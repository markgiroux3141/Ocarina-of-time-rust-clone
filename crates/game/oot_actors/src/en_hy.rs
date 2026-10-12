//! `En_Hy` (`ovl_En_Hy/z_en_hy.c`): the Hylian townsfolk. `params & 0x7F` is the type (21:
//! `EnHyType`), `(params >> 7) & 0xF` the path (15 for none).
//!
//! Each type is a head (`sHeadInfo`: its object, list and eyes) on a skeleton (`sSkeletonInfo`,
//! the upper and lower body always the same object), animated from `object_os_anime`
//! (`sAnimationInfo`), with env colours for its clothes on segments 8 and 9 (and 10 for the young
//! women's hair) by type (`sModelInfo`). It waits for its three objects (`EnHy_WaitForObjects`),
//! then:
//! - the bald man with his bag walks his path (`EnHy_Walk`), the old man paces his to and fro
//!   once Link is near (`EnHy_SetupPace`, `EnHy_Pace`), the dog lady in the market watches her dog
//!   (`EnHy_WatchDog`), the others fidget (`EnHy_Fidget`) or stand (`EnHy_DoNothing`);
//! - every one turns its head (and torso, or body) to Link (`Npc_TrackPoint` with its type's
//!   preset) and talks (`Npc_UpdateTalking`): its text by type, scene and story
//!   (`EnHy_GetTextId`), and the flags its first texts set as they close (`EnHy_UpdateTalkState`);
//! - the beggar buys bottled blue fire, fish and bugs (`EnHy_OfferBuyBottledItem`: 150, 100 and 50
//!   rupees); the dog lady gives a piece of heart (then a blue rupee) for Richard
//!   (`EnHy_WaitDogFoundRewardGiven`).
//!
//! In the market by day and the back alley by day they lose `ACTOR_FLAG_UPDATE_CULLING_DISABLED`
//! and their culling volume's scale (0): out of view they stand still.
//!
//! Drawn as `EnHy_Draw` does: the skeleton shifted by the type's model offset, the head limb's list
//! the head's (`EnHy_OverrideLimbDraw`, from the head's object on segment 6) with its eyes on
//! segment 10, the head and torso turns and the fidget's sway, the bald man's bag after the torso
//! (`EnHy_PostLimbDraw`); from meshes baked per type and eye (docs/adr/0012-actor-bakes.md).
//!
//! Not ported here: the mask reactions (`MaskReaction_GetTextId`: no masks yet, so none), the
//! circle shadow's size (`shadowScale` kept), and `Player_UpdateBottleHeld` (the beggar's bottle
//! emptied in Link's hand: Player's bottles aren't ported; logged).

use std::cell::Cell;
use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, MeshKey};
use eng_math::{binang_to_rad, cos_s, sin_s, smooth_step_to_s};
use glam::{Mat4, Vec3};
use oot_game::actor::{ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_FRIENDLY, ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor, UPDBGCHECKINFO_FLAG_2};
use oot_game::actor_ctx::{ACTORCAT_NPC, ActorImpl, ActorProfile};
use oot_game::collision_check::*;
use oot_game::get_item::{actor_has_parent, offer_get_item_range};
use oot_game::item::{GI_HEART_PIECE, ITEM_BOTTLE_BLUE_FIRE, rupees_change_by};
use oot_game::message::*;
use oot_game::npc::{NpcTrack, actor_update_fidget_tables, talk_update, track_point};
use oot_game::pack::{BakeBody, BakeSegment, LimbOverride, MeshBake, keys};
use oot_game::path::{path_get_by_index, path_orient_and_get_dist_sq};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::skelanime_std::*;

/// `ACTOR_EN_HY` (`actor_table.h`: 0x016E).
pub const ACTOR_EN_HY: i16 = 0x016E;

/// `En_Hy_Profile`.
pub const PROFILE: ActorProfile =
    ActorProfile { id: ACTOR_EN_HY, name: "En_Hy", category: ACTORCAT_NPC, flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY | ACTOR_FLAG_UPDATE_CULLING_DISABLED, object: "gameplay_keep" };

// `EnHyType`.
pub const ENHY_TYPE_DOG_LADY: i16 = 0;
pub const ENHY_TYPE_WOMAN_3: i16 = 1;
pub const ENHY_TYPE_MAN_1_BEARD: i16 = 2;
pub const ENHY_TYPE_MAN_2_BALD: i16 = 3;
pub const ENHY_TYPE_MAN_1_SHAVED_BLACK_SHIRT: i16 = 4;
pub const ENHY_TYPE_BEGGAR: i16 = 5;
pub const ENHY_TYPE_OLD_WOMAN: i16 = 6;
pub const ENHY_TYPE_OLD_MAN: i16 = 7;
pub const ENHY_TYPE_YOUNG_WOMAN_BROWN_HAIR: i16 = 8;
pub const ENHY_TYPE_MAN_2_MUSTACHE_RED_SHIRT: i16 = 9;
pub const ENHY_TYPE_MAN_2_MUSTACHE_BLUE_SHIRT: i16 = 10;
pub const ENHY_TYPE_YOUNG_WOMAN_ORANGE_HAIR: i16 = 11;
pub const ENHY_TYPE_MAN_2_ALT_MUSTACHE: i16 = 12;
pub const ENHY_TYPE_MAN_1_BOWL_CUT_PURPLE_SHIRT: i16 = 13;
pub const ENHY_TYPE_MAN_2_BEARD: i16 = 14;
pub const ENHY_TYPE_OLD_MAN_BALD_BROWN_ROBE: i16 = 15;
pub const ENHY_TYPE_MAN_2_MUSTACHE_WHITE_SHIRT: i16 = 16;
pub const ENHY_TYPE_MAN_1_SHAVED_GREEN_SHIRT: i16 = 17;
pub const ENHY_TYPE_WOMAN_2: i16 = 18;
pub const ENHY_TYPE_OLD_MAN_BALD_PURPLE_ROBE: i16 = 19;
pub const ENHY_TYPE_MAN_1_BOWL_CUT_GREEN_SHIRT: i16 = 20;
pub const ENHY_TYPE_MAX: i16 = 21;

// `EnHyLimb` (1-based, as `limbIndex`).
pub const ENHY_LIMB_RIGHT_FOOT: usize = 7;
pub const ENHY_LIMB_TORSO: usize = 8;
pub const ENHY_LIMB_LEFT_UPPER_ARM: usize = 9;
pub const ENHY_LIMB_RIGHT_UPPER_ARM: usize = 12;
pub const ENHY_LIMB_HEAD: usize = 15;
pub const ENHY_LIMB_MAX: usize = 16;

// `OBJECT_*` (`object_table.h`).
const OBJECT_OS_ANIME: i16 = 0x00C5;
const OBJECT_AOB: i16 = 0x0105;
const OBJECT_AHG: i16 = 0x0107;
const OBJECT_CNE: i16 = 0x0108;
const OBJECT_BJI: i16 = 0x010C;
const OBJECT_BBA: i16 = 0x010D;
const OBJECT_BOJ: i16 = 0x0111;
const OBJECT_BOB: i16 = 0x0115;
const OBJECT_COB: i16 = 0x018C;

/// An object's id and file name.
fn object_file(id: i16) -> &'static str {
    match id {
        OBJECT_AOB => "object_aob",
        OBJECT_AHG => "object_ahg",
        OBJECT_CNE => "object_cne",
        OBJECT_BJI => "object_bji",
        OBJECT_BBA => "object_bba",
        OBJECT_BOJ => "object_boj",
        OBJECT_BOB => "object_bob",
        OBJECT_COB => "object_cob",
        _ => "object_os_anime",
    }
}

// `SCENE_*` (`scene_table.h`).
const SCENE_KAKARIKO_CENTER_GUEST_HOUSE: u16 = 0x2A;
const SCENE_DOG_LADY_HOUSE: u16 = 0x35;
const SCENE_IMPAS_HOUSE: u16 = 0x37;
const SCENE_BACK_ALLEY_DAY: u16 = 0x1E;
const SCENE_BACK_ALLEY_NIGHT: u16 = 0x1F;
const SCENE_MARKET_DAY: u16 = 0x20;
const SCENE_KAKARIKO_VILLAGE: u16 = 0x52;

// `EVENTCHKINF_*`, `INFTABLE_*`, `EVENTINF_*` (`save.h`).
const EVENTCHKINF_TALKED_TO_MALON_FIRST_TIME: u16 = 0x10;
const EVENTCHKINF_TALON_RETURNED_FROM_CASTLE: u16 = 0x14;
const EVENTCHKINF_OBTAINED_MASTER_SWORD: u16 = 0x45;
const EVENTCHKINF_TALON_RETURNED_FROM_KAKARIKO: u16 = 0x6B;
const EVENTCHKINF_ZELDA_FLED_CASTLE: u16 = 0x80;
const EVENTCHKINF_AA: u16 = 0xAA;
const INFTABLE_MALON_SPAWNED_AT_HYRULE_CASTLE: u16 = 0x8B;
const INFTABLE_C0: u16 = 0xC0;
const INFTABLE_C1: u16 = 0xC1;
const INFTABLE_C2: u16 = 0xC2;
const INFTABLE_C3: u16 = 0xC3;
const INFTABLE_C4: u16 = 0xC4;
const INFTABLE_C5: u16 = 0xC5;
const INFTABLE_C6: u16 = 0xC6;
const INFTABLE_C7: u16 = 0xC7;
const INFTABLE_C8: u16 = 0xC8;
const INFTABLE_C9: u16 = 0xC9;
const INFTABLE_CA: u16 = 0xCA;
const INFTABLE_CB: u16 = 0xCB;
const INFTABLE_CC: u16 = 0xCC;
const INFTABLE_CD: u16 = 0xCD;
const INFTABLE_160: u16 = 0x160;
const INFTABLE_161: u16 = 0x161;
const INFTABLE_162: u16 = 0x162;
const INFTABLE_163: u16 = 0x163;
const INFTABLE_164: u16 = 0x164;
const INFTABLE_191: u16 = 0x191;
/// The dog lady's dog is out in the market (`EVENTINF_30`).
pub const EVENTINF_30: u16 = 0x30;
/// `QUEST_MEDALLION_SHADOW` (`item.h`).
const QUEST_MEDALLION_SHADOW: u32 = 0x04;

// `EXCH_ITEM_*` (`item.h`: 0x18 to 0x1D).
const EXCH_ITEM_BOTTLE_FISH: u8 = 0x18;
const EXCH_ITEM_BOTTLE_BLUE_FIRE: u8 = 0x19;
const EXCH_ITEM_BOTTLE_BUG: u8 = 0x1A;
const EXCH_ITEM_BOTTLE_POE: u8 = 0x1B;
const EXCH_ITEM_BOTTLE_BIG_POE: u8 = 0x1C;
const EXCH_ITEM_BOTTLE_RUTOS_LETTER: u8 = 0x1D;
/// `ITEM_BOTTLE_FISH`, `ITEM_BOTTLE_BUG` (`item.h`: 0x19, 0x1D).
const ITEM_BOTTLE_FISH: u8 = 0x19;
const ITEM_BOTTLE_BUG: u8 = 0x1D;
/// `GI_RUPEE_BLUE` (`item.h`: 0x4D).
const GI_RUPEE_BLUE: i16 = 0x4D;
use oot_game::audio::NA_BGM_ITEM_GET;

// `NpcTrackingMode`, `NpcTalkState` (`actor.h`).
const NPC_TRACKING_NONE: i16 = 1;
const NPC_TRACKING_HEAD_AND_TORSO: i16 = 3;
const NPC_TRACKING_FULL_BODY: i16 = 4;
const NPC_TALK_STATE_IDLE: i16 = 0;
const NPC_TALK_STATE_TALKING: i16 = 1;
const NPC_TALK_STATE_ACTION: i16 = 2;

/// `FIDGET_AMPLITUDE` (`actor.h`).
const FIDGET_AMPLITUDE: f32 = 200.0;

/// `sColliderCylinderInit`.
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

/// `sColChkInfoInit`.
const COL_CHK_INFO_INIT: CollisionCheckInfoInit2 = CollisionCheckInfoInit2 { health: 0, cyl_radius: 0, cyl_height: 0, cyl_y_shift: 0, mass: MASS_IMMOVABLE };

/// `EnHyHeadInfo`: the head's object, list and eye textures (`sHylian*EyeTextures`, `NULL`
/// ended: here without the `NULL`).
struct HeadInfo {
    object: i16,
    dlist: &'static str,
    eyes: Option<&'static [&'static str]>,
}

const WOMAN_1_EYES: &[&str] = &["gHylianWoman1EyeOpenTex", "gHylianWoman1EyeHalfTex", "gHylianWoman1EyeClosedTex"];
const MAN_1_EYES: &[&str] = &["gHylianMan1BeardedEyeOpenTex", "gHylianMan1BeardedEyeHalfTex", "gHylianMan1BeardedEyeClosedTex"];
const OLD_WOMAN_EYES: &[&str] = &["gHylianOldWomanEyeTex"];
const OLD_MAN_EYES: &[&str] = &["gHylianOldManEyeOpenTex", "gHylianOldManEyeHalfTex", "gHylianOldManEyeClosedTex"];
const MAN_2_EYES: &[&str] = &["gHylianMan2MustachedEyeOpenTex", "gHylianMan2MustachedEyeHalfTex", "gHylianMan2MustachedEyeClosedTex"];
const WOMAN_2_EYES: &[&str] = &["gHylianWoman2EyeOpenTex", "gHylianWoman2EyeHalfTex", "gHylianWoman2EyeClosedTex"];

/// `sHeadInfo`, by `EnHyHeadIndex`.
const HEAD_INFO: [HeadInfo; 16] = [
    HeadInfo { object: OBJECT_AOB, dlist: "gHylianWoman1HeadDL", eyes: Some(WOMAN_1_EYES) },
    HeadInfo { object: OBJECT_BOB, dlist: "gHylianWoman2HeadDL", eyes: Some(WOMAN_2_EYES) },
    HeadInfo { object: OBJECT_BOJ, dlist: "gHylianMan2MustachedHeadDL", eyes: Some(MAN_2_EYES) },
    HeadInfo { object: OBJECT_BOJ, dlist: "gHylianMan2BaldHeadDL", eyes: None },
    HeadInfo { object: OBJECT_BOJ, dlist: "gHylianMan2LongHairHeadDL", eyes: None },
    HeadInfo { object: OBJECT_BOJ, dlist: "gHylianMan2AltMustachedHeadDL", eyes: None },
    HeadInfo { object: OBJECT_BOJ, dlist: "gHylianMan2BeardedHeadDL", eyes: None },
    HeadInfo { object: OBJECT_AHG, dlist: "gHylianMan1BeardedHeadDL", eyes: Some(MAN_1_EYES) },
    HeadInfo { object: OBJECT_AHG, dlist: "gHylianMan1ShavedHeadDL", eyes: None },
    HeadInfo { object: OBJECT_AHG, dlist: "gHylianMan1BowlCutHeadDL", eyes: None },
    HeadInfo { object: OBJECT_BBA, dlist: "gHylianOldWomanHeadDL", eyes: Some(OLD_WOMAN_EYES) },
    HeadInfo { object: OBJECT_CNE, dlist: "gHylianYoungWomanBrownHairHeadDL", eyes: None },
    HeadInfo { object: OBJECT_CNE, dlist: "gHylianYoungWomanOrangeHairHeadDL", eyes: None },
    HeadInfo { object: OBJECT_BJI, dlist: "gHylianOldManHeadDL", eyes: Some(OLD_MAN_EYES) },
    HeadInfo { object: OBJECT_BJI, dlist: "gHylianOldManBaldHeadDL", eyes: None },
    HeadInfo { object: OBJECT_COB, dlist: "gHylianWoman3HeadDL", eyes: None },
];

/// `sSkeletonInfo`, by `EnHySkeletonIndex`: the object and its flex skeleton.
const SKELETON_INFO: [(i16, &str); 8] = [
    (OBJECT_AOB, "gHylianWoman1Skel"),
    (OBJECT_BOB, "gHylianWoman2Skel"),
    (OBJECT_BOJ, "gHylianMan2Skel"),
    (OBJECT_AHG, "gHylianMan1Skel"),
    (OBJECT_BBA, "gHylianOldWomanSkel"),
    (OBJECT_CNE, "gHylianYoungWomanSkel"),
    (OBJECT_BJI, "gHylianOldManSkel"),
    (OBJECT_COB, "gHylianWoman3Skel"),
];

/// `sAnimationInfo` (`EnHyAnimationIndex` 0..26), from `object_os_anime`: animation, speed,
/// start, end, mode, morph frames.
const ANIMATION_INFO: [(&str, f32, f32, f32, u8, f32); 27] = [
    ("gObjOsAnim_092C", 1.0, 0.0, -1.0, ANIMMODE_LOOP, 0.0),
    ("gObjOsAnim_0228", 1.0, 0.0, -1.0, ANIMMODE_LOOP, 0.0),
    ("gObjOsAnim_4CF4", 1.0, 0.0, -1.0, ANIMMODE_LOOP, 0.0),
    ("gObjOsAnim_16EC", 1.0, 0.0, -1.0, ANIMMODE_LOOP, 0.0),
    ("gObjOsAnim_265C", 1.0, 0.0, -1.0, ANIMMODE_LOOP, 0.0),
    ("gObjOsAnim_42AC", 1.0, 0.0, -1.0, ANIMMODE_LOOP, 0.0),
    ("gObjOsAnim_28DC", 1.0, 0.0, -1.0, ANIMMODE_LOOP, 0.0),
    ("gObjOsAnim_2160", 1.0, 0.0, -1.0, ANIMMODE_LOOP, -10.0),
    ("gObjOsAnim_265C", 1.0, 0.0, -1.0, ANIMMODE_LOOP, -10.0),
    ("gObjOsAnim_4E90", 1.0, 0.0, -1.0, ANIMMODE_LOOP, 0.0),
    ("gObjOsAnim_1E7C", 1.0, 0.0, -1.0, ANIMMODE_LOOP, 0.0),
    ("gObjOsAnim_0170", 1.0, 0.0, -1.0, ANIMMODE_LOOP, 0.0),
    ("gObjOsAnim_00B4", 1.0, 0.0, -1.0, ANIMMODE_LOOP, 0.0),
    ("gObjOsAnim_3D84", 1.0, 0.0, -1.0, ANIMMODE_LOOP, 0.0),
    ("gObjOsAnim_41F8", 1.0, 0.0, -1.0, ANIMMODE_LOOP, 0.0),
    ("gObjOsAnim_300C", 1.0, 0.0, -1.0, ANIMMODE_LOOP, 0.0),
    ("gObjOsAnim_31B0", 1.0, 0.0, -1.0, ANIMMODE_LOOP, 0.0),
    ("gObjOsAnim_31B0", 1.0, 0.0, -1.0, ANIMMODE_LOOP, -8.0),
    ("gObjOsAnim_2D0C", 1.0, 0.0, -1.0, ANIMMODE_LOOP, 0.0),
    ("gObjOsAnim_2DC0", 1.0, 0.0, -1.0, ANIMMODE_LOOP, 0.0),
    ("gObjOsAnim_4408", 1.0, 0.0, -1.0, ANIMMODE_LOOP, 0.0),
    ("gObjOsAnim_1F18", 1.0, 0.0, -1.0, ANIMMODE_LOOP, 0.0),
    ("gObjOsAnim_4F28", 1.0, 0.0, -1.0, ANIMMODE_LOOP, 0.0),
    ("gObjOsAnim_33B4", 1.0, 0.0, -1.0, ANIMMODE_LOOP, -8.0),
    ("gObjOsAnim_12E8", 1.0, 0.0, -1.0, ANIMMODE_LOOP, -8.0),
    ("gObjOsAnim_0FE4", 1.0, 0.0, -1.0, ANIMMODE_LOOP, -8.0),
    ("gObjOsAnim_0BFC", 1.0, 0.0, -1.0, ANIMMODE_LOOP, -8.0),
];
const ENHY_ANIM_7: usize = 7;
const ENHY_ANIM_8: usize = 8;
const ENHY_ANIM_17: usize = 17;
const ENHY_ANIM_23: usize = 23;
const ENHY_ANIM_24: usize = 24;
const ENHY_ANIM_25: usize = 25;
const ENHY_ANIM_26: usize = 26;

/// `EnHyModelInfo`: head index, upper skeleton index, segment 8's env colour, lower skeleton
/// index, segment 9's env colour, animation index.
struct ModelInfo {
    head: usize,
    upper_skel: usize,
    env8: [u8; 4],
    lower_skel: usize,
    env9: [u8; 4],
    anim: usize,
}

const fn mi(head: usize, upper_skel: usize, env8: [u8; 4], lower_skel: usize, env9: [u8; 4], anim: usize) -> ModelInfo {
    ModelInfo { head, upper_skel, env8, lower_skel, env9, anim }
}

/// `sModelInfo`, by type.
const MODEL_INFO: [ModelInfo; 21] = [
    mi(0, 0, [255, 255, 255, 255], 0, [255, 255, 255, 255], 0),
    mi(15, 7, [255, 255, 255, 255], 7, [255, 255, 255, 255], 22),
    mi(7, 3, [255, 255, 255, 255], 3, [255, 255, 255, 255], 1),
    mi(3, 2, [255, 255, 255, 0], 2, [55, 55, 255, 0], 15),
    mi(8, 3, [0, 0, 0, 0], 3, [255, 0, 0, 0], 11),
    mi(4, 2, [50, 80, 0, 0], 2, [50, 80, 0, 0], 16),
    mi(10, 4, [255, 255, 255, 255], 4, [255, 255, 255, 255], 10),
    mi(13, 6, [0, 50, 160, 0], 6, [255, 255, 255, 0], 4),
    mi(11, 5, [160, 180, 255, 0], 5, [160, 180, 255, 0], 9),
    mi(2, 2, [220, 0, 80, 0], 2, [255, 255, 255, 0], 13),
    mi(2, 2, [0, 130, 220, 0], 2, [255, 255, 255, 0], 14),
    mi(12, 5, [70, 160, 230, 0], 5, [255, 255, 100, 0], 20),
    mi(5, 2, [150, 60, 90, 0], 2, [255, 240, 150, 0], 18),
    mi(9, 3, [200, 180, 255, 0], 3, [200, 180, 255, 0], 12),
    mi(6, 2, [140, 255, 110, 0], 2, [255, 255, 255, 0], 19),
    mi(14, 6, [130, 70, 20, 0], 6, [130, 180, 255, 0], 21),
    mi(2, 2, [255, 255, 255, 255], 2, [255, 255, 255, 255], 5),
    mi(8, 3, [90, 100, 20, 255], 3, [100, 140, 50, 255], 11),
    mi(1, 1, [255, 255, 255, 255], 1, [255, 255, 255, 255], 6),
    mi(14, 6, [160, 0, 100, 0], 6, [70, 130, 210, 0], 21),
    mi(9, 3, [160, 230, 0, 0], 3, [0, 150, 110, 0], 12),
];

/// `sColliderInfo`: the collider's offset from the actor, radius and height, by type.
const COLLIDER_INFO: [([i16; 3], i16, i16); 21] = [
    ([0, 0, 4], 24, 70),
    ([0, 0, 8], 28, 62),
    ([0, 0, 4], 20, 60),
    ([0, 0, 2], 20, 60),
    ([0, 0, -2], 20, 60),
    ([0, 0, 8], 24, 40),
    ([0, 0, 10], 26, 40),
    ([0, 0, 12], 26, 58),
    ([0, 0, 2], 18, 68),
    ([0, 0, 4], 20, 60),
    ([0, 0, 4], 20, 60),
    ([0, 0, 6], 20, 64),
    ([0, 0, 0], 18, 60),
    ([0, 0, 0], 16, 60),
    ([0, 0, 0], 16, 64),
    ([0, 0, 8], 20, 58),
    ([4, 0, 0], 18, 62),
    ([4, 0, 0], 18, 62),
    ([0, 0, 8], 28, 62),
    ([0, 0, 0], 16, 60),
    ([0, 0, 8], 20, 58),
];

/// `sPlayerTrackingInfo`: the tracking preset and the child's and adult's eye height, by type.
const PLAYER_TRACKING_INFO: [(usize, f32, f32); 21] = [
    (6, 20.0, 10.0),
    (6, 20.0, 10.0),
    (7, 40.0, 20.0),
    (6, 20.0, 10.0),
    (7, 40.0, 20.0),
    (8, 0.0, -20.0),
    (9, 20.0, 0.0),
    (9, 20.0, 0.0),
    (6, 20.0, 10.0),
    (6, 20.0, 10.0),
    (6, 20.0, 10.0),
    (6, 20.0, 10.0),
    (0, 0.0, 0.0),
    (6, 20.0, 10.0),
    (6, 20.0, 10.0),
    (10, 20.0, 0.0),
    (6, 20.0, 10.0),
    (6, 20.0, 10.0),
    (6, 20.0, 10.0),
    (6, 20.0, 10.0),
    (10, 20.0, 0.0),
];

/// `sInit2Info`: shadow scale, model offset, scale, attention range, interact range, by type.
const INIT2_INFO: [(f32, [f32; 3], f32, u8, f32); 21] = [
    (36.0, [0.0, 0.0, 600.0], 0.01, 6, 30.0),
    (40.0, [-100.0, 0.0, 400.0], 0.01, 6, 30.0),
    (22.0, [0.0, 0.0, -200.0], 0.01, 6, 30.0),
    (20.0, [-100.0, 0.0, 0.0], 0.01, 6, 30.0),
    (22.0, [0.0, 0.0, 0.0], 0.01, 6, 30.0),
    (21.0, [0.0, 0.0, 0.0], 0.01, 6, 30.0),
    (25.0, [-100.0, 0.0, 600.0], 0.01, 6, 30.0),
    (28.0, [-100.0, 0.0, 800.0], 0.01, 6, 30.0),
    (17.0, [0.0, 0.0, 700.0], 0.01, 6, 30.0),
    (18.0, [0.0, 0.0, 100.0], 0.01, 6, 30.0),
    (18.0, [0.0, 0.0, -200.0], 0.01, 6, 30.0),
    (17.0, [0.0, 0.0, 700.0], 0.01, 6, 30.0),
    (21.0, [0.0, 0.0, -300.0], 0.01, 6, 30.0),
    (20.0, [0.0, 0.0, -200.0], 0.01, 6, 30.0),
    (18.0, [-200.0, 0.0, -200.0], 0.01, 6, 30.0),
    (27.0, [-100.0, 0.0, 800.0], 0.01, 6, 30.0),
    (19.0, [400.0, 0.0, 0.0], 0.01, 4, 30.0),
    (19.0, [400.0, 0.0, 0.0], 0.01, 6, 30.0),
    (40.0, [-100.0, 0.0, 400.0], 0.01, 6, 30.0),
    (17.0, [0.0, 0.0, 700.0], 0.01, 6, 30.0),
    (20.0, [0.0, 0.0, -200.0], 0.01, 6, 30.0),
];

/// `gDPSetEnvColor(r, g, b, a)` (`G_SETENVCOLOR` 0xFB), as `EnHy_SetEnvColor`'s list.
fn set_env_color(c: [u8; 4]) -> (u32, u32) {
    (0xFB00_0000, u32::from_be_bytes(c))
}

/// Whether `EnHy_Draw` sets segments 8 and 9 for the type (all but the dog lady, the third woman,
/// the old woman and the second woman).
fn sets_env_colors(ty: i16) -> bool {
    !matches!(ty, ENHY_TYPE_DOG_LADY | ENHY_TYPE_WOMAN_3 | ENHY_TYPE_OLD_WOMAN | ENHY_TYPE_WOMAN_2)
}

/// The number of eye textures the type's head has (1 without: its single bake).
pub fn eye_count(ty: i16) -> usize {
    HEAD_INFO[MODEL_INFO[ty as usize].head].eyes.map_or(1, |e| e.len())
}

/// The bake of a type with an eye.
pub fn bake_name(ty: i16, eye: usize) -> String {
    format!("En_Hy/type{ty}_eye{eye}")
}

/// The bald man's bag (`gHylianMan2BagDL`), drawn after his torso.
pub const BAG_BAKE: &str = "En_Hy/bag";

/// The meshes `EnHy_Draw` draws: each type's skeleton with its head, colours and each eye, and
/// the bag.
pub fn bakes() -> Vec<MeshBake> {
    let mut v = Vec::new();
    for ty in 0..ENHY_TYPE_MAX {
        let m = &MODEL_INFO[ty as usize];
        let head = &HEAD_INFO[m.head];
        let (skel_object, skel) = SKELETON_INFO[m.lower_skel];
        for eye in 0..eye_count(ty) {
            let mut segments = Vec::new();
            if sets_env_colors(ty) {
                segments.push((0x08, BakeSegment::Commands(vec![set_env_color(m.env8)])));
                segments.push((0x09, BakeSegment::Commands(vec![set_env_color(m.env9)])));
                match ty {
                    ENHY_TYPE_YOUNG_WOMAN_BROWN_HAIR => segments.push((0x0A, BakeSegment::Commands(vec![set_env_color(m.env8)]))),
                    ENHY_TYPE_YOUNG_WOMAN_ORANGE_HAIR => segments.push((0x0A, BakeSegment::Commands(vec![set_env_color([255, 255, 255, 0])]))),
                    _ => {}
                }
            }
            if let Some(eyes) = head.eyes {
                segments.push((0x0A, BakeSegment::Texture { file: object_file(head.object).into(), symbol: eyes[eye].into() }));
            }
            v.push(MeshBake {
                name: bake_name(ty, eye),
                object: object_file(skel_object).into(),
                segments,
                prelude: Vec::new(),
                body: BakeBody::Skeleton {
                    file: object_file(skel_object).into(),
                    symbol: skel.into(),
                    limbs: vec![LimbOverride { limb: (ENHY_LIMB_HEAD - 1) as u8, file: object_file(head.object).into(), symbol: head.dlist.into() }],
                },
            });
        }
    }
    let m = &MODEL_INFO[ENHY_TYPE_MAN_2_BALD as usize];
    v.push(MeshBake {
        name: BAG_BAKE.into(),
        object: "object_boj".into(),
        segments: vec![(0x08, BakeSegment::Commands(vec![set_env_color(m.env8)])), (0x09, BakeSegment::Commands(vec![set_env_color(m.env9)]))],
        prelude: Vec::new(),
        body: BakeBody::DLists(vec![("object_boj".into(), "gHylianMan2BagDL".into())]),
    });
    v
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    WaitForObjects,
    Walk,
    SetupPace,
    Pace,
    WatchDog,
    Fidget,
    DoNothing,
    WaitDogFoundRewardGiven,
    FinishGivingDogFoundReward,
}

pub struct EnHy {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    pub skeleton: Option<Arc<Skeleton>>,
    pub action: Action,
    /// `pathReverse`: pacing back along the path.
    pub path_reverse: bool,
    /// `objectSlotHead`, `objectSlotUpperSkel`, `objectSlotLowerSkel`, `objectSlotOsAnime`.
    pub object_slot_head: Option<usize>,
    pub object_slot_upper_skel: Option<usize>,
    pub object_slot_lower_skel: Option<usize>,
    pub object_slot_os_anime: Option<usize>,
    pub collider: ColliderCylinder,
    pub interact_info: NpcTrack,
    /// `path`: the scene path's index.
    pub path: Option<usize>,
    pub waypoint: i8,
    pub played_sfx: bool,
    pub cur_eye_index: i16,
    pub next_eye_index_timer: i16,
    pub fidget_table_y: [i16; ENHY_LIMB_MAX],
    pub fidget_table_z: [i16; ENHY_LIMB_MAX],
    pub interact_range: f32,
    pub get_item_id: i16,
    pub model_offset: Vec3,
    /// `talonEventChkInf`: the guest house's word of `eventChkInf` with Talon's return, at its init.
    pub talon_event_chk_inf: u16,
    animations: Vec<AnimationInfo>,
}

impl EnHy {
    fn ty(&self) -> i16 {
        self.actor.params & 0x7F
    }

    fn model(&self) -> &'static ModelInfo {
        &MODEL_INFO[self.ty() as usize]
    }

    /// `EnHy_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut this = EnHy {
            actor,
            skel: SkelAnimeStd::init_flex(ENHY_LIMB_MAX - 1, None),
            skeleton: None,
            action: Action::WaitForObjects,
            path_reverse: false,
            object_slot_head: None,
            object_slot_upper_skel: None,
            object_slot_lower_skel: None,
            object_slot_os_anime: None,
            collider: ColliderCylinder::new(&CYLINDER_INIT),
            interact_info: NpcTrack::default(),
            path: None,
            waypoint: 0,
            played_sfx: false,
            cur_eye_index: 0,
            next_eye_index_timer: 0,
            fidget_table_y: [0; ENHY_LIMB_MAX],
            fidget_table_z: [0; ENHY_LIMB_MAX],
            interact_range: 0.0,
            get_item_id: 0,
            model_offset: Vec3::ZERO,
            talon_event_chk_inf: 0,
            animations: Vec::new(),
        };
        if this.ty() >= ENHY_TYPE_MAX || !this.find_os_anime_object(play) || !this.find_skel_and_head_objects(play) {
            this.actor.kill();
        }
        if !this.should_spawn(play) {
            this.actor.kill();
        }
        this.action = Action::WaitForObjects;
        Box::new(this)
    }

    /// `EnHy_FindSkelAndHeadObjects`.
    fn find_skel_and_head_objects(&mut self, play: &PlayState) -> bool {
        if self.ty() >= ENHY_TYPE_MAX {
            return false;
        }
        let m = self.model();
        self.object_slot_lower_skel = play.object_ctx.get_index(SKELETON_INFO[m.lower_skel].0);
        if self.object_slot_lower_skel.is_none() {
            return false;
        }
        self.object_slot_upper_skel = play.object_ctx.get_index(SKELETON_INFO[m.upper_skel].0);
        if self.object_slot_upper_skel.is_none() {
            return false;
        }
        self.object_slot_head = play.object_ctx.get_index(HEAD_INFO[m.head].object);
        self.object_slot_head.is_some()
    }

    /// `EnHy_AreSkelAndHeadObjectsLoaded`.
    fn are_skel_and_head_objects_loaded(&self, play: &PlayState) -> bool {
        [self.object_slot_lower_skel, self.object_slot_upper_skel, self.object_slot_head].iter().all(|s| s.is_some_and(|b| play.object_ctx.is_loaded(b)))
    }

    /// `EnHy_FindOsAnimeObject`.
    fn find_os_anime_object(&mut self, play: &PlayState) -> bool {
        self.object_slot_os_anime = play.object_ctx.get_index(OBJECT_OS_ANIME);
        self.object_slot_os_anime.is_some()
    }

    /// `EnHy_IsOsAnimeObjectLoaded`.
    fn is_os_anime_object_loaded(&self, play: &PlayState) -> bool {
        self.object_slot_os_anime.is_some_and(|b| play.object_ctx.is_loaded(b))
    }

    /// `Animation_ChangeByInfo(&skelAnime, sAnimationInfo, i)`.
    fn change_anim(&mut self, i: usize) {
        if let Some(info) = self.animations.get(i) {
            self.skel.change_by_info(info);
        }
    }

    /// `EnHy_GiveItem`.
    fn give_item(&mut self, play: &mut PlayState, get_item_id: i16) {
        self.get_item_id = get_item_id;
        let a = self.actor.clone();
        offer_get_item_range(play, &a, get_item_id, self.actor.xz_dist_to_player + 1.0, self.actor.y_dist_to_player.abs() + 1.0);
    }

    /// `EnHy_UpdateEyes`.
    fn update_eyes(&mut self, play: &mut PlayState) {
        // DECR(nextEyeIndexTimer) == 0.
        if self.next_eye_index_timer != 0 {
            self.next_eye_index_timer -= 1;
            if self.next_eye_index_timer != 0 {
                return;
            }
        }
        self.cur_eye_index = self.cur_eye_index.wrapping_add(1);
        // A head without eye textures counts on forever (its index is never read).
        if let Some(eyes) = HEAD_INFO[self.model().head].eyes
            && self.cur_eye_index as usize >= eyes.len()
        {
            self.next_eye_index_timer = play.rand.s16_offset(30, 30);
            self.cur_eye_index = 0;
        }
    }

    /// `EnHy_InitCollider`.
    fn init_collider(&mut self) {
        let (_, radius, height) = COLLIDER_INFO[self.ty() as usize];
        self.collider.dim.radius = radius;
        self.collider.dim.height = height;
    }

    /// `EnHy_InitSetProperties`.
    fn init_set_properties(&mut self) {
        let (_shadow_scale, offset, scale, attention_range, interact_range) = INIT2_INFO[self.ty() as usize];
        self.actor.scale = Vec3::splat(scale);
        self.actor.target_mode = attention_range;
        self.model_offset = Vec3::from_array(offset);
        self.interact_range = interact_range + self.collider.dim.radius as f32;
    }

    /// `EnHy_UpdateCollider`: at the actor's position (truncated) plus the type's offset.
    fn update_collider(&mut self, play: &mut PlayState) {
        let (offset, _, _) = COLLIDER_INFO[self.ty() as usize];
        let p = self.actor.world_pos;
        self.collider.dim.pos = [(p.x as i16).wrapping_add(offset[0]), (p.y as i16).wrapping_add(offset[1]), (p.z as i16).wrapping_add(offset[2])];
        play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
    }

    /// `EnHy_OfferBuyBottledItem`: the beggar's text for what Link shows him.
    fn offer_buy_bottled_item(&mut self, play: &mut PlayState) {
        if self.ty() != ENHY_TYPE_BEGGAR {
            return;
        }
        let player = play.player.and_then(|h| play.actors.get(h)).and_then(|p| p.as_player());
        let (exchange, mask) = player.map(|p| (p.exchange_item_id(), p.current_mask())).unwrap_or((0, 0));
        let s = &play.save;
        use oot_game::item::inventory_has_specific_bottle as has;
        if !has(s, ITEM_BOTTLE_BLUE_FIRE) && !has(s, ITEM_BOTTLE_BUG) && !has(s, ITEM_BOTTLE_FISH) {
            match exchange {
                EXCH_ITEM_BOTTLE_POE | EXCH_ITEM_BOTTLE_BIG_POE | EXCH_ITEM_BOTTLE_RUTOS_LETTER => self.actor.text_id = 0x70EF,
                _ => {
                    if mask == 0 {
                        self.actor.text_id = 0x70ED;
                    }
                }
            }
        } else {
            match exchange {
                EXCH_ITEM_BOTTLE_BLUE_FIRE => self.actor.text_id = 0x70F0,
                EXCH_ITEM_BOTTLE_FISH => self.actor.text_id = 0x70F1,
                EXCH_ITEM_BOTTLE_BUG => self.actor.text_id = 0x70F2,
                _ => {
                    if mask == 0 {
                        self.actor.text_id = 0x700C;
                    }
                }
            }
        }
        let text_id = self.actor.text_id;
        if let Some(a) = play.player.and_then(|h| play.actors.actor_mut(h)) {
            a.text_id = text_id;
        }
    }

    /// `EnHy_UpdateNPC`: the tracking and the talk.
    fn update_npc(&mut self, play: &mut PlayState) {
        let ty = self.ty();
        let talk_idle = self.interact_info.talk_state == NPC_TALK_STATE_IDLE;
        let tracking_mode = match ty {
            ENHY_TYPE_MAN_2_BALD | ENHY_TYPE_OLD_MAN | ENHY_TYPE_MAN_2_MUSTACHE_RED_SHIRT | ENHY_TYPE_MAN_2_MUSTACHE_BLUE_SHIRT => {
                if talk_idle {
                    NPC_TRACKING_NONE
                } else {
                    NPC_TRACKING_HEAD_AND_TORSO
                }
            }
            ENHY_TYPE_MAN_2_ALT_MUSTACHE => NPC_TRACKING_NONE,
            ENHY_TYPE_MAN_1_BEARD | ENHY_TYPE_MAN_1_SHAVED_GREEN_SHIRT => NPC_TRACKING_FULL_BODY,
            ENHY_TYPE_DOG_LADY | ENHY_TYPE_WOMAN_2 => {
                if talk_idle {
                    NPC_TRACKING_HEAD_AND_TORSO
                } else {
                    NPC_TRACKING_FULL_BODY
                }
            }
            _ => NPC_TRACKING_HEAD_AND_TORSO,
        };
        let (preset, child_y, adult_y) = PLAYER_TRACKING_INFO[ty as usize];
        self.interact_info.target = play.player.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos).unwrap_or(self.actor.world_pos);
        self.interact_info.eye_height = if play.save.adult { adult_y } else { child_y };
        let mut t = self.interact_info;
        track_point(play, &mut self.actor, &mut t, preset, tracking_mode);
        self.interact_info = t;

        // Npc_UpdateTalking(.., EnHy_GetTextId, EnHy_UpdateTalkState): the callbacks read and
        // write the actor's own state through these cells.
        let talon = self.talon_event_chk_inf;
        let played_sfx = Cell::new(self.played_sfx);
        let set_exchange = Cell::new(false);
        let anim_change = Cell::new(None::<usize>);
        let give = Cell::new(None::<i16>);
        let anim_is_33b4 = self.skel.is("gObjOsAnim_33B4");
        let mut talk_state = self.interact_info.talk_state;
        let range = self.interact_range;
        let started = talk_update(
            play,
            &mut self.actor,
            &mut talk_state,
            range,
            |play, _| {
                let (text, sfx_reset, exch) = en_hy_get_text_id(play, ty, talon);
                if sfx_reset {
                    played_sfx.set(false);
                }
                set_exchange.set(exch);
                text
            },
            |play, actor| {
                let mut ps = played_sfx.get();
                let mut anim = None;
                let mut gi = None;
                let r = en_hy_update_talk_state(play, actor, &mut ps, anim_is_33b4, &mut anim, &mut gi);
                played_sfx.set(ps);
                anim_change.set(anim);
                give.set(gi);
                r
            },
        );
        self.interact_info.talk_state = talk_state;
        self.played_sfx = played_sfx.get();
        if set_exchange.get()
            && let Some(p) = play.player.and_then(|h| play.actors.get_mut(h)).and_then(|p| p.as_player_mut())
        {
            p.set_exchange_item_id(EXCH_ITEM_BOTTLE_BLUE_FIRE);
        }
        if let Some(i) = anim_change.get() {
            self.change_anim(i);
        }
        if let Some(gi) = give.get() {
            self.give_item(play, gi);
            self.action = Action::WaitDogFoundRewardGiven;
        }
        if started {
            self.offer_buy_bottled_item(play);
        }
    }

    /// `EnHy_ShouldSpawn`.
    fn should_spawn(&self, play: &PlayState) -> bool {
        let ty = self.ty();
        let s = &play.save;
        let (adult, day) = (s.adult, s.is_day());
        match play.scene_id {
            SCENE_KAKARIKO_VILLAGE => {
                if !matches!(ty, ENHY_TYPE_MAN_2_MUSTACHE_RED_SHIRT | ENHY_TYPE_MAN_2_MUSTACHE_BLUE_SHIRT | ENHY_TYPE_MAN_2_ALT_MUSTACHE | ENHY_TYPE_MAN_1_BEARD | ENHY_TYPE_OLD_MAN) {
                    true
                } else if !adult {
                    true
                } else {
                    !(ty != ENHY_TYPE_MAN_2_ALT_MUSTACHE && !day)
                }
            }
            SCENE_IMPAS_HOUSE => {
                if ty != ENHY_TYPE_MAN_2_MUSTACHE_BLUE_SHIRT {
                    true
                } else if !adult {
                    false
                } else {
                    !day
                }
            }
            SCENE_DOG_LADY_HOUSE => ty != ENHY_TYPE_DOG_LADY || !day,
            SCENE_KAKARIKO_CENTER_GUEST_HOUSE => {
                if ty == ENHY_TYPE_DOG_LADY {
                    adult
                } else if !matches!(ty, ENHY_TYPE_MAN_2_MUSTACHE_RED_SHIRT | ENHY_TYPE_MAN_1_BEARD | ENHY_TYPE_OLD_MAN) {
                    true
                } else if day {
                    false
                } else {
                    adult
                }
            }
            SCENE_BACK_ALLEY_DAY | SCENE_BACK_ALLEY_NIGHT => {
                if ty != ENHY_TYPE_MAN_2_BEARD {
                    true
                } else if !day {
                    false
                } else {
                    !(s.get_event_chk_inf(EVENTCHKINF_ZELDA_FLED_CASTLE) && !s.get_event_chk_inf(EVENTCHKINF_OBTAINED_MASTER_SWORD))
                }
            }
            _ => !(matches!(ty, ENHY_TYPE_OLD_MAN_BALD_PURPLE_ROBE | ENHY_TYPE_MAN_1_BOWL_CUT_GREEN_SHIRT) && adult),
        }
    }

    /// `EnHy_WaitForObjects`: once its objects are in, the skeleton, the collider, the type's
    /// animation and properties, and its action.
    fn wait_for_objects(&mut self, play: &mut PlayState) {
        if !(self.is_os_anime_object_loaded(play) && self.are_skel_and_head_objects_loaded(play)) {
            return;
        }
        self.actor.obj_bank_index = self.object_slot_lower_skel;
        let ty = self.ty();
        let m = self.model();
        let Some(assets) = play.assets.clone() else { return };
        let loaded = assets.skeleton(object_file(SKELETON_INFO[m.lower_skel].0), SKELETON_INFO[m.lower_skel].1).and_then(|s| {
            let animations = ANIMATION_INFO
                .iter()
                .map(|&(name, play_speed, start_frame, frame_count, mode, morph_frames)| {
                    assets.animation("object_os_anime", name).map(|animation| AnimationInfo { animation, play_speed, start_frame, frame_count, mode, morph_frames })
                })
                .collect::<anyhow::Result<Vec<_>>>()?;
            Ok((s, animations))
        });
        let (skeleton, animations) = match loaded {
            Ok(x) => x,
            Err(e) => {
                log::error!("En_Hy: {e:#}");
                self.actor.kill();
                return;
            }
        };
        // SkelAnime_InitFlex(.., ENHY_LIMB_MAX).
        self.skel = SkelAnimeStd::init_flex(skeleton.limbs.len(), None);
        self.skeleton = Some(skeleton);
        self.animations = animations;
        // ActorShape_Init(&shape, 0, ActorShadow_DrawCircle, 0).
        self.actor.shape_y_offset = 0.0;
        self.collider = ColliderCylinder::new(&CYLINDER_INIT);
        self.init_collider();
        self.actor.col_chk_info.set_info2(None, &COL_CHK_INFO_INIT);
        self.change_anim(m.anim);
        if play.scene_id == SCENE_BACK_ALLEY_DAY || play.scene_id == SCENE_MARKET_DAY {
            self.actor.flags &= !ACTOR_FLAG_UPDATE_CULLING_DISABLED;
            self.actor.culling_volume_scale = 0.0;
        }
        if play.scene_id == SCENE_KAKARIKO_CENTER_GUEST_HOUSE {
            // eventChkInf[EVENTCHKINF_INDEX_TALON_RETURNED_FROM_KAKARIKO].
            self.talon_event_chk_inf = play.save.event_chk_inf[(EVENTCHKINF_TALON_RETURNED_FROM_KAKARIKO >> 4) as usize];
        }
        self.init_set_properties();
        self.path = path_get_by_index(play.setup_path_list(), (self.actor.params >> 7) & 0xF, 15);
        self.action = match ty {
            ENHY_TYPE_MAN_2_BALD => {
                if self.path.is_some() {
                    self.actor.speed_xz = 3.0;
                }
                Action::Walk
            }
            ENHY_TYPE_OLD_MAN => {
                self.path_reverse = false;
                Action::SetupPace
            }
            ENHY_TYPE_DOG_LADY if play.scene_id == SCENE_MARKET_DAY => Action::WatchDog,
            ENHY_TYPE_DOG_LADY
            | ENHY_TYPE_WOMAN_3
            | ENHY_TYPE_MAN_1_BEARD
            | ENHY_TYPE_MAN_1_SHAVED_BLACK_SHIRT
            | ENHY_TYPE_OLD_WOMAN
            | ENHY_TYPE_YOUNG_WOMAN_BROWN_HAIR
            | ENHY_TYPE_MAN_1_BOWL_CUT_PURPLE_SHIRT
            | ENHY_TYPE_MAN_2_BEARD
            | ENHY_TYPE_OLD_MAN_BALD_BROWN_ROBE
            | ENHY_TYPE_MAN_2_MUSTACHE_WHITE_SHIRT
            | ENHY_TYPE_MAN_1_SHAVED_GREEN_SHIRT
            | ENHY_TYPE_WOMAN_2
            | ENHY_TYPE_OLD_MAN_BALD_PURPLE_ROBE
            | ENHY_TYPE_MAN_1_BOWL_CUT_GREEN_SHIRT => Action::Fidget,
            ENHY_TYPE_BEGGAR | ENHY_TYPE_MAN_2_MUSTACHE_RED_SHIRT | ENHY_TYPE_MAN_2_MUSTACHE_BLUE_SHIRT | ENHY_TYPE_YOUNG_WOMAN_ORANGE_HAIR | ENHY_TYPE_MAN_2_ALT_MUSTACHE => Action::DoNothing,
            _ => {
                self.actor.kill();
                Action::DoNothing
            }
        };
    }

    /// `EnHy_WatchDog`.
    fn watch_dog(&mut self, play: &PlayState) {
        if self.interact_info.talk_state != NPC_TALK_STATE_IDLE {
            if !self.skel.is("gObjOsAnim_0BFC") {
                self.change_anim(ENHY_ANIM_26);
            }
        } else if play.save.get_event_inf(EVENTINF_30) {
            if !self.skel.is("gObjOsAnim_0FE4") {
                self.change_anim(ENHY_ANIM_25);
            }
        } else if !self.skel.is("gObjOsAnim_12E8") {
            self.change_anim(ENHY_ANIM_24);
        }
    }

    fn path<'a>(&self, play: &'a PlayState) -> Option<&'a oot_game::scene::Path> {
        self.path.and_then(|i| play.setup_path_list().get(i))
    }

    /// `EnHy_Walk`.
    fn walk(&mut self, play: &PlayState) {
        let path = self.path(play);
        let (dist_sq, yaw) = path_orient_and_get_dist_sq(&self.actor, path, self.waypoint as i16);
        // @bug (game): without a path the yaw is never written, and the C turns towards whatever
        // was on its stack; here it doesn't turn.
        if let Some(yaw) = yaw {
            smooth_step_to_s(&mut self.actor.world_rot.y, yaw, 10, 1000, 1);
        }
        self.actor.shape_rot = self.actor.world_rot;
        if dist_sq > 0.0 && dist_sq < 1000.0 {
            self.waypoint += 1;
            if let Some(p) = path
                && self.waypoint as usize > p.count() - 1
            {
                self.waypoint = 0;
            }
        }
    }

    /// `EnHy_Fidget`.
    fn fidget(&mut self, play: &PlayState) {
        actor_update_fidget_tables(play, &mut self.fidget_table_y, &mut self.fidget_table_z, ENHY_LIMB_MAX);
    }

    /// `EnHy_SetupPace`.
    fn setup_pace(&mut self, play: &PlayState) {
        if self.actor.xz_dist_to_player <= 100.0 && self.path.is_some() {
            self.change_anim(ENHY_ANIM_7);
            self.actor.speed_xz = 0.4;
            self.action = Action::Pace;
        }
        actor_update_fidget_tables(play, &mut self.fidget_table_y, &mut self.fidget_table_z, ENHY_LIMB_MAX);
    }

    /// `EnHy_Pace`: to and fro along his path.
    fn pace(&mut self, play: &PlayState) {
        if self.skel.is("gObjOsAnim_2160") && self.interact_info.talk_state != NPC_TALK_STATE_IDLE {
            self.change_anim(ENHY_ANIM_8);
        }
        if self.skel.is("gObjOsAnim_265C") && self.interact_info.talk_state == NPC_TALK_STATE_IDLE {
            self.change_anim(ENHY_ANIM_7);
        }
        self.actor.speed_xz = 0.4;
        let path = self.path(play);
        let (dist_sq, yaw) = path_orient_and_get_dist_sq(&self.actor, path, self.waypoint as i16);
        if let Some(yaw) = yaw {
            smooth_step_to_s(&mut self.actor.world_rot.y, yaw, 10, 1000, 1);
        }
        self.actor.shape_rot = self.actor.world_rot;
        if !(dist_sq <= 0.0) && !(dist_sq >= 1000.0) {
            let count = path.map_or(0, |p| p.count()) as i8;
            if !self.path_reverse {
                self.waypoint += 1;
                if self.waypoint > count - 1 {
                    self.path_reverse = true;
                    self.waypoint = count - 2;
                }
            } else {
                self.waypoint -= 1;
                if self.waypoint < 0 {
                    self.path_reverse = false;
                    self.waypoint = 1;
                }
            }
        }
    }

    /// `EnHy_WaitDogFoundRewardGiven`.
    fn wait_dog_found_reward_given(&mut self, play: &mut PlayState) {
        if actor_has_parent(&self.actor) {
            self.action = Action::FinishGivingDogFoundReward;
        } else {
            let a = self.actor.clone();
            offer_get_item_range(play, &a, self.get_item_id, self.actor.xz_dist_to_player + 1.0, self.actor.y_dist_to_player.abs() + 1.0);
        }
    }

    /// `EnHy_FinishGivingDogFoundReward`.
    fn finish_giving_dog_found_reward(&mut self, play: &mut PlayState) {
        if play.message_state() == TEXT_STATE_DONE && play.message_should_advance() {
            match self.get_item_id {
                GI_HEART_PIECE => {
                    play.save.dog_params = 0;
                    play.save.dog_is_lost = false;
                    play.save.set_inf_table(INFTABLE_191);
                }
                GI_RUPEE_BLUE => {
                    rupees_change_by(&mut play.save, 5);
                    play.save.dog_params = 0;
                    play.save.dog_is_lost = false;
                }
                _ => {}
            }
            self.action = Action::Fidget;
        }
    }

    /// The limb matrices `EnHy_Draw` draws with: the pose, and `EnHy_OverrideLimbDraw`'s head and
    /// torso turns and the fidget's sway of the torso and the upper arms.
    fn pose(skeleton: &Skeleton, joints: &[[i16; 3]], head: [i16; 2], torso: [i16; 2], sway: &[(usize, i16, i16)]) -> Vec<Mat4> {
        let r = binang_to_rad;
        skeleton.pose_override(joints, |limb, _pos, rot| {
            let mut pre = Mat4::IDENTITY;
            if limb == ENHY_LIMB_HEAD {
                // Translate(1400, 0, 0), RotateX(headRot.y), RotateZ(headRot.x), Translate(-1400, 0, 0).
                pre =
                    Mat4::from_translation(Vec3::new(1400.0, 0.0, 0.0)) * Mat4::from_rotation_x(r(head[1])) * Mat4::from_rotation_z(r(head[0])) * Mat4::from_translation(Vec3::new(-1400.0, 0.0, 0.0));
            }
            if limb == ENHY_LIMB_TORSO {
                // RotateX(-torsoRot.y), RotateZ(torsoRot.x).
                pre = Mat4::from_rotation_x(r(torso[1].wrapping_neg())) * Mat4::from_rotation_z(r(torso[0]));
            }
            if let Some(&(_, a, b)) = sway.iter().find(|s| s.0 == limb) {
                rot[1] = (rot[1] as f32 + sin_s(a) * FIDGET_AMPLITUDE) as i32 as i16;
                rot[2] = (rot[2] as f32 + cos_s(b) * FIDGET_AMPLITUDE) as i32 as i16;
            }
            pre
        })
    }

    fn sway_of(a: &[i16; ENHY_LIMB_MAX], b: &[i16; ENHY_LIMB_MAX]) -> [(usize, i16, i16); 3] {
        [ENHY_LIMB_TORSO, ENHY_LIMB_LEFT_UPPER_ARM, ENHY_LIMB_RIGHT_UPPER_ARM].map(|l| (l, a[l], b[l]))
    }

    /// The model matrix `EnHy_Draw` draws the skeleton at: `Actor_Draw`'s, then
    /// `Matrix_Translate(modelOffset)`.
    fn model_matrix(rs: &RenderState, offset: Vec3) -> Mat4 {
        oot_game::play::actor_draw_matrix(rs) * Mat4::from_translation(offset)
    }
}

/// `EnHy_GetTextId`: the type's text for the scene and the story. Returns the text, whether it
/// clears `playedSfx` (the dog lady with a dog following), and whether it sets Player's
/// `exchangeItemId` to `EXCH_ITEM_BOTTLE_BLUE_FIRE` (the beggar).
fn en_hy_get_text_id(play: &PlayState, ty: i16, talon_event_chk_inf: u16) -> (u16, bool, bool) {
    // MaskReaction_GetTextId(play, type + MASK_REACTION_SET_DOG_LADY): no masks, so 0.
    let s = &play.save;
    let fled = s.get_event_chk_inf(EVENTCHKINF_ZELDA_FLED_CASTLE);
    let talon_back = talon_event_chk_inf & (1 << (EVENTCHKINF_TALON_RETURNED_FROM_KAKARIKO & 0xF)) != 0;
    let t = |b: bool, x: u16, y: u16| if b { x } else { y };
    let text = match ty {
        ENHY_TYPE_DOG_LADY => {
            if play.scene_id == SCENE_KAKARIKO_CENTER_GUEST_HOUSE {
                t(talon_back, 0x508D, t(s.get_inf_table(INFTABLE_CB), 0x508C, 0x508B))
            } else if play.scene_id == SCENE_MARKET_DAY {
                t(s.get_event_inf(EVENTINF_30), 0x709B, 0x709C)
            } else if s.dog_is_lost {
                let following_dog = (s.dog_params & 0xF00) >> 8;
                if following_dog != 0 {
                    return (t(following_dog == 1, 0x709F, 0x709E), true, false);
                }
                0x709D
            } else {
                0x70A0
            }
        }
        ENHY_TYPE_WOMAN_3 => {
            if fled {
                t(s.get_inf_table(INFTABLE_C1), 0x7017, 0x7045)
            } else {
                t(s.get_inf_table(INFTABLE_C0), 0x7017, 0x7016)
            }
        }
        ENHY_TYPE_MAN_1_BEARD => {
            if play.scene_id == SCENE_KAKARIKO_CENTER_GUEST_HOUSE {
                0x5086
            } else if play.scene_id == SCENE_KAKARIKO_VILLAGE {
                0x5085
            } else if fled {
                t(s.get_inf_table(INFTABLE_C3), 0x701A, 0x7047)
            } else if s.get_event_chk_inf(EVENTCHKINF_TALON_RETURNED_FROM_CASTLE) {
                0x701A
            } else if s.get_event_chk_inf(EVENTCHKINF_TALKED_TO_MALON_FIRST_TIME) {
                0x701B
            } else if s.get_inf_table(INFTABLE_C2) {
                0x701C
            } else {
                0x701A
            }
        }
        ENHY_TYPE_MAN_2_BALD => t(fled, t(s.get_inf_table(INFTABLE_C4), 0x7001, 0x70EB), 0x7001),
        ENHY_TYPE_MAN_1_SHAVED_BLACK_SHIRT => t(fled, 0x704B, t(s.get_inf_table(INFTABLE_C5), 0x7024, 0x7023)),
        ENHY_TYPE_BEGGAR => return (0x700C, false, true),
        ENHY_TYPE_OLD_WOMAN => t(fled, 0x704A, t(s.get_inf_table(INFTABLE_C6), 0x7022, 0x7021)),
        ENHY_TYPE_OLD_MAN => {
            if play.scene_id == SCENE_KAKARIKO_CENTER_GUEST_HOUSE {
                0x5088
            } else if play.scene_id == SCENE_KAKARIKO_VILLAGE {
                0x5087
            } else {
                t(fled, 0x704D, t(s.get_inf_table(INFTABLE_C7), 0x7028, 0x7027))
            }
        }
        ENHY_TYPE_YOUNG_WOMAN_BROWN_HAIR => {
            if fled {
                t(s.get_inf_table(INFTABLE_C9), 0x701E, 0x7048)
            } else {
                t(s.get_inf_table(INFTABLE_C8), 0x701E, 0x701D)
            }
        }
        ENHY_TYPE_MAN_2_MUSTACHE_RED_SHIRT => {
            if play.scene_id == SCENE_KAKARIKO_CENTER_GUEST_HOUSE {
                t(s.get_event_chk_inf(EVENTCHKINF_AA), 0x5082, 0x5081)
            } else if play.scene_id == SCENE_KAKARIKO_VILLAGE {
                t(s.check_quest_item(QUEST_MEDALLION_SHADOW), 0x5080, 0x507F)
            } else {
                t(fled, 0x7049, t(s.get_inf_table(INFTABLE_CA), 0x7020, 0x701F))
            }
        }
        ENHY_TYPE_MAN_2_MUSTACHE_BLUE_SHIRT => {
            if play.scene_id == SCENE_IMPAS_HOUSE {
                t(s.get_event_chk_inf(EVENTCHKINF_AA), 0x507E, 0x507D)
            } else if play.scene_id == SCENE_KAKARIKO_VILLAGE {
                t(s.check_quest_item(QUEST_MEDALLION_SHADOW), 0x507C, 0x507B)
            } else {
                // OOT_VERSION >= NTSC_1_1 (this ROM): INFTABLE_CD.
                t(fled, 0x7046, t(s.get_inf_table(INFTABLE_CD), 0x7019, 0x7018))
            }
        }
        ENHY_TYPE_YOUNG_WOMAN_ORANGE_HAIR => t(s.get_inf_table(INFTABLE_MALON_SPAWNED_AT_HYRULE_CASTLE), t(s.get_inf_table(INFTABLE_CC), 0x7014, 0x70A4), 0x7014),
        ENHY_TYPE_MAN_2_ALT_MUSTACHE => {
            if play.scene_id == SCENE_KAKARIKO_VILLAGE {
                t(!s.is_day(), 0x5084, 0x5083)
            } else {
                t(fled, 0x7044, 0x7015)
            }
        }
        ENHY_TYPE_MAN_1_BOWL_CUT_PURPLE_SHIRT => 0x7055,
        ENHY_TYPE_MAN_2_BEARD => 0x7089,
        ENHY_TYPE_OLD_MAN_BALD_BROWN_ROBE => 0x708A,
        ENHY_TYPE_MAN_2_MUSTACHE_WHITE_SHIRT => 0x700E,
        ENHY_TYPE_MAN_1_SHAVED_GREEN_SHIRT => {
            if !s.adult {
                if s.is_day() { t(s.get_inf_table(INFTABLE_160), 0x5058, 0x5057) } else { t(s.get_inf_table(INFTABLE_161), 0x505A, 0x5059) }
            } else if s.is_day() {
                t(s.get_inf_table(INFTABLE_162), 0x505C, 0x505B)
            } else {
                0x5058
            }
        }
        ENHY_TYPE_WOMAN_2 => {
            if !s.adult {
                t(fled, 0x505F, t(s.get_inf_table(INFTABLE_163), 0x505E, 0x505D))
            } else {
                t(talon_back, 0x5062, t(s.get_inf_table(INFTABLE_164), 0x5061, 0x5060))
            }
        }
        ENHY_TYPE_OLD_MAN_BALD_PURPLE_ROBE => 0x7120,
        ENHY_TYPE_MAN_1_BOWL_CUT_GREEN_SHIRT => 0x7121,
        _ => 0,
    };
    (text, false, false)
}

/// `EnHy_UpdateTalkState`: the talk's state each frame (`Npc_UpdateTalking`'s second callback), the
/// dog lady's chime and the beggar's fanfare, and the flags the first texts set as they close.
/// `anim` takes an animation change for the actor, `give` the dog lady's reward.
fn en_hy_update_talk_state(play: &mut PlayState, actor: &mut Actor, played_sfx: &mut bool, anim_is_33b4: bool, anim: &mut Option<usize>, give: &mut Option<i16>) -> i16 {
    // beggarRewards (the unused beggarItems: blue fire, fish, bug, fairy).
    const BEGGAR_REWARDS: [i16; 4] = [150, 100, 50, 25];
    match play.message_state() {
        TEXT_STATE_NONE | TEXT_STATE_DONE_HAS_NEXT | TEXT_STATE_CHOICE | TEXT_STATE_DONE | TEXT_STATE_SONG_DEMO_DONE | TEXT_STATE_8 | TEXT_STATE_9 => NPC_TALK_STATE_TALKING,
        TEXT_STATE_DONE_FADING => {
            match actor.text_id {
                0x709E | 0x709F => {
                    if !*played_sfx {
                        use oot_game::audio::sfx::{NA_SE_SY_CORRECT_CHIME, NA_SE_SY_ERROR};
                        play.audio.play_sfx_centered(if actor.text_id == 0x709F { NA_SE_SY_CORRECT_CHIME } else { NA_SE_SY_ERROR });
                        *played_sfx = true;
                    }
                }
                0x70F0..=0x70F3 => {
                    if !anim_is_33b4 {
                        *anim = Some(ENHY_ANIM_23);
                        play.audio.play_fanfare(NA_BGM_ITEM_GET | 0x900);
                    }
                }
                _ => {}
            }
            NPC_TALK_STATE_TALKING
        }
        TEXT_STATE_CLOSING => {
            let s = &mut play.save;
            match actor.text_id {
                0x70F0..=0x70F3 => {
                    rupees_change_by(s, BEGGAR_REWARDS[(actor.text_id - 0x70F0) as usize]);
                    *anim = Some(ENHY_ANIM_17);
                    // Player_UpdateBottleHeld(play, player, ITEM_BOTTLE_EMPTY, PLAYER_IA_BOTTLE).
                    log::info!("En_Hy: Player_UpdateBottleHeld (the beggar's bottle emptied) isn't ported");
                }
                0x7016 => s.set_inf_table(INFTABLE_C0),
                0x7045 => s.set_inf_table(INFTABLE_C1),
                0x701B => s.set_inf_table(INFTABLE_C2),
                0x7047 => s.set_inf_table(INFTABLE_C3),
                0x70EB => s.set_inf_table(INFTABLE_C4),
                0x7023 => s.set_inf_table(INFTABLE_C5),
                0x7021 => s.set_inf_table(INFTABLE_C6),
                0x7027 => s.set_inf_table(INFTABLE_C7),
                0x701D => s.set_inf_table(INFTABLE_C8),
                0x7048 => s.set_inf_table(INFTABLE_C9),
                0x701F => s.set_inf_table(INFTABLE_CA),
                // OOT_VERSION >= NTSC_1_1 (this ROM): INFTABLE_CD.
                0x7018 => s.set_inf_table(INFTABLE_CD),
                0x70A4 => s.set_inf_table(INFTABLE_CC),
                0x5057 => s.set_inf_table(INFTABLE_160),
                0x5059 => s.set_inf_table(INFTABLE_161),
                0x505B => s.set_inf_table(INFTABLE_162),
                0x505D => s.set_inf_table(INFTABLE_163),
                0x5060 => s.set_inf_table(INFTABLE_164),
                0x508B => s.set_inf_table(INFTABLE_CB),
                0x709E => s.dog_params = 0,
                0x709F => *give = Some(if s.get_inf_table(INFTABLE_191) { GI_RUPEE_BLUE } else { GI_HEART_PIECE }),
                _ => {}
            }
            NPC_TALK_STATE_IDLE
        }
        TEXT_STATE_EVENT => {
            if !play.message_should_advance() {
                NPC_TALK_STATE_TALKING
            } else {
                NPC_TALK_STATE_ACTION
            }
        }
        _ => NPC_TALK_STATE_TALKING,
    }
}

/// Indices into the render state's extras.
mod rs {
    /// `angles`: head x, y; torso x, y; the sway (torso, left and right upper arms).
    pub const HEAD: usize = 0;
    pub const TORSO: usize = 2;
    pub const SWAY: usize = 4;
    /// `switches`: the eye.
    pub const EYE: usize = 0;
}

impl ActorImpl for EnHy {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `EnHy_Update`.
    fn update(&mut self, play: &mut PlayState) {
        if self.action != Action::WaitForObjects {
            self.skel.update();
            self.update_eyes(play);
            if self.interact_info.talk_state == NPC_TALK_STATE_IDLE {
                self.actor.move_forward();
            }
            self.actor.update_bg_check_info(&play.col, 0.0, 0.0, 0.0, UPDBGCHECKINFO_FLAG_2);
        }
        match self.action {
            Action::WaitForObjects => self.wait_for_objects(play),
            Action::Walk => self.walk(play),
            Action::SetupPace => self.setup_pace(play),
            Action::Pace => self.pace(play),
            Action::WatchDog => self.watch_dog(play),
            Action::Fidget => self.fidget(play),
            Action::DoNothing => {}
            Action::WaitDogFoundRewardGiven => self.wait_dog_found_reward_given(play),
            Action::FinishGivingDogFoundReward => self.finish_giving_dog_found_reward(play),
        }
        if self.actor.killed {
            return;
        }
        self.update_npc(play);
        self.update_collider(play);
    }
    /// `EnHy_PostLimbDraw`, the head: the focus 400 along it.
    fn draw_update(&mut self, _play: &mut PlayState) {
        let Some(skeleton) = &self.skeleton else { return };
        let t = &self.interact_info;
        let bones = Self::pose(skeleton, &self.skel.joint_table, [t.head[0], t.head[1]], [t.torso[0], t.torso[1]], &Self::sway_of(&self.fidget_table_y, &self.fidget_table_z));
        let m = Self::model_matrix(&RenderState::of(&self.actor), self.model_offset);
        if let Some(head) = bones.get(ENHY_LIMB_HEAD - 1) {
            self.actor.focus_pos = (m * *head).transform_point3(Vec3::new(400.0, 0.0, 0.0));
        }
    }
    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        if self.action == Action::WaitForObjects {
            return rs;
        }
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        let t = &self.interact_info;
        let sway = Self::sway_of(&self.fidget_table_y, &self.fidget_table_z);
        rs.angles = vec![t.head[0], t.head[1], t.torso[0], t.torso[1], sway[0].1, sway[0].2, sway[1].1, sway[1].2, sway[2].1, sway[2].2];
        let eye = if HEAD_INFO[self.model().head].eyes.is_some() { self.cur_eye_index.max(0) as u32 } else { 0 };
        rs.switches = vec![eye];
        rs
    }
    /// `EnHy_Draw`.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let (Some(skeleton), Some(joints)) = (&self.skeleton, &rs.joints) else { return };
        if rs.angles.len() < 10 || rs.switches.is_empty() {
            return;
        }
        let a = &rs.angles;
        let sway = [(ENHY_LIMB_TORSO, a[rs::SWAY], a[rs::SWAY + 1]), (ENHY_LIMB_LEFT_UPPER_ARM, a[rs::SWAY + 2], a[rs::SWAY + 3]), (ENHY_LIMB_RIGHT_UPPER_ARM, a[rs::SWAY + 4], a[rs::SWAY + 5])];
        let bones = Self::pose(skeleton, &joints.rot, [a[rs::HEAD], a[rs::HEAD + 1]], [a[rs::TORSO], a[rs::TORSO + 1]], &sway);
        let m = Self::model_matrix(rs, self.model_offset);
        let ty = self.ty();
        let eye = (rs.switches[rs::EYE] as usize).min(eye_count(ty) - 1);
        if ty == ENHY_TYPE_MAN_2_BALD
            && let Some(torso) = bones.get(ENHY_LIMB_TORSO - 1)
        {
            out.opa.push(DrawCmd::new(MeshKey::named(keys::bake(BAG_BAKE)), m * *torso));
        }
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(&bake_name(ty, eye))), transform: m, bones, params: Default::default() });
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
