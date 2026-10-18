//! `En_Go2` (`ovl_En_Go2/z_en_go2.c`): the Gorons of Death Mountain Trail, Goron City and the
//! market's bazaar (`GoronType`, the params' low five bits).
//!
//! **Curled up** (`EnGo2_CurledUp`, drawn as `gGoronDL_00BD80`, the rock) most of them sleep
//! until Link is within 200 (800 for Biggoron), less than 60 above or below and in front of
//! them (`EnGo2_IsWakingUp`); then they uncurl (`EnGo2_WakeUp`, `gGoronAnim_004930` forwards,
//! `NA_SE_EN_GOLON_WAKE_UP`), sit up (`func_80A46B40`, `func_80A454CC`: some with
//! `gGoronAnim_0021D0`), turn their head and chest to Link (`Npc_TrackPoint`, preset 4) and talk
//! (`Npc_UpdateTalking`). When Link leaves they curl up again (`EnGo2_RollingAnimation`: the same
//! animation backwards), unless `EnGo2_IsCameraModified` (the big rolling Goron, the stairwell's,
//! the bazaar's; anyone while Link owns the Goron Tunic without the Fire Medallion).
//!
//! **Rolling** (`EnGo2_SetupRolling`: `shape.yOffset` 1800, drawn as `gGoronDL_00C140` spun by
//! `play->state.frames`): ten frames spinning in place (`EnGo2_ContinueRolling`), then along the
//! scene path of the params (`EnGo2_SlowRolling`), turning to each point and on to the next within
//! 30 (`EnGo2_Orient`, `EnGo2_UpdateWaypoint`), bouncing (`EnGo2_IsRollingOnGround`) and raising
//! dust (`EnGo2_SpawnDust`, ten `EnGoEffect`s drawn with `gGoronDL_00FD40`/`FD50` and `gDust1-8Tex`).
//! Touching him knocks Link down at 1.5 times his speed and costs 4 (`func_80A44AB0`). The waypoints
//! run from 0 to the path's count less two, then back to 0 (the last point is never aimed for).
//! - The trail's small one (`GORON_DMT_ROLLING_SMALL`) holds until Link is within 1000, rolls at
//!   6 and can't be stopped; back at waypoint 0 (the path's second-to-last point reached) he stops
//!   (`EnGo2_StopRolling`) and drops a bomb that blows at once (`En_Bom`, timer 0), bounces to a
//!   stop (`EnGo2_GroundRolling`) and curls up.
//! - Goron City's big one (`GORON_CITY_ROLLING_BIG`) rolls his path for ever (3.6, starting at 6
//!   once he's given his gift); his cylinder takes only `DMG_EXPLOSIVE` while he rolls: a blast
//!   stops him (`NA_SE_SY_CORRECT_CHIME`), he settles, wakes and offers to talk (`Actor_OfferTalk`):
//!   0x3012 when the bomb bag holds 20 and he aims for waypoint 8 to 11, which gives the 30 bomb bag
//!   (the 40 with the 30: `GI_BOMB_BAG_30`/`_40`, `INFTABLE_11E`), 0x3013 after it, else 0x3011;
//!   then he curls up and rolls on (`EnGo2_GoronRollingBigContinueRolling`).
//!
//! **The bomb-flower Goron** (`GORON_DMT_BOMB_FLOWER`): 0x300A's choice, "yes" 0x300B with the
//! bracelet (`INFTABLE_EB` as it closes, and he walks his path: `EnGo2_GoronDmtBombFlowerAnimation`)
//! or 0x300C without, "no" 0x300D. With `INFTABLE_EB` his init puts him at his path's last point.
//!
//! Drawn as `EnGo2_Draw` does: the dust first, then curled (`EnGo2_DrawCurledUp`), rolling
//! (`EnGo2_DrawRolling`) or the skeleton `gGoronSkel` with the eyes on segment 8 and the mouth on 9
//! (`EnGo2_OverrideLimbDraw`'s head and chest turns and the sway of limbs 10, 11 and 14;
//! `EnGo2_PostLimbDraw`'s focus), from meshes baked per eye and mouth
//! (docs/adr/0012-actor-bakes.md). The circle shadow is the shape's (`ActorShape_Init`'s, drawn by
//! `Actor_Draw`), its scale by type (`EnGo2_SetShape`), its alpha faded by `func_80A45360` (none
//! while curled).
//!
//! **Ported:** the init (every type's kills), the data tables, the dust, the talk dispatch and
//! every child's type's texts and flags (`EnGo2_GetTextId*`, `EnGo2_UpdateTalkState*`), the bazaar's,
//! the adult's generic texts (0x3043), the collision and shape, the waypoints, waking and curling,
//! the rolling states, the get item, the update and the draw.
//!
//! **Not ported** (the adult's Gorons, Phase 15): types 1 (Link the Goron, `GORON_CITY_LINK`), 2
//! (Biggoron, `GORON_DMT_BIGGORON`) and 3 (the Fire Temple's, `GORON_FIRE_GENERIC`) are placeholders
//! from their init's own case on (a type 3 whose switch flag is set is killed first, as the C
//! does). Their functions: `EnGo2_GoronFireGenericGetTextId`, `EnGo2_GetTextIdGoronCityLink`,
//! `EnGo2_UpdateTalkStateGoronCityLink`, `EnGo2_GetTextIdGoronDmtBiggoron`,
//! `EnGo2_UpdateTalkStateGoronDmtBiggoron`, `EnGo2_GetTextIdGoronFireGeneric`,
//! `EnGo2_UpdateTalkStateGoronFireGeneric`, `EnGo2_GetDialogState` (theirs only),
//! `EnGo2_BiggoronSetTextId`, `EnGo2_IsFreeingGoronInFire`, `EnGo2_IsGoronFireGeneric`,
//! `EnGo2_IsGoronLinkReversing`, `EnGo2_GoronLinkAnimation`, `EnGo2_GoronFireCamera`,
//! `EnGo2_GoronFireClearCamera`, `EnGo2_BiggoronAnimation`, `EnGo2_ReverseRolling`,
//! `EnGo2_BiggoronEyedrops`, `EnGo2_GoronLinkStopRolling`, `EnGo2_GoronFireGenericAction` and
//! `func_80A44D84`; Biggoron's quake in `EnGo2_CurledUp`, his camera in `EnGo2_IsCameraModified`,
//! one-point cutscenes 4190 and 4200, and `EnGo2_SetGetItem`'s Goron Tunic, Biggoron's Sword and
//! claim check cases. Where a shared function branches on those types the branch logs
//! (`log::debug!`). Also not ported: the mask reactions (`MaskReaction_GetTextId(play,
//! MASK_REACTION_SET_GORON)`: no masks yet, so none).
//!
//! Faithful bugs: Link the Goron's stop in `EnGo2_SlowRolling` waits for `EnGo2_Orient` to return
//! 2, which it never does; a type over 0xD would read past the data tables (killed here).
//!
//! Params: `GoronType` bits 0 to 4 (below); `path` bits 5 to 9 (the scene's path he rolls or walks:
//! the trail's rolling Goron path 1, the bomb-flower Goron and the city's big one path 0; 0x1F
//! none); `switchFlag` bits 10 to 15 (the Fire Temple Goron's: set, he's freed; the others have 0x3F
//! or anything, unread). `shape.rot.z` low byte (`unk_216`): Link the Goron's waypoint from which
//! he turns back (placed 10).
//! Flags: reads EVENTCHKINF_23 (the cavern's boulder blown: the trail's texts); reads and sets
//! INFTABLE_E0 (0x3008 closed), INFTABLE_E3 (0x300E), INFTABLE_E6 (0x3024), INFTABLE_EB (0x300B:
//! the bomb-flower Goron moved), INFTABLE_F0 (0x3014), INFTABLE_F4 (0x3016), INFTABLE_F8 (0x3018)
//! and INFTABLE_11E (the bomb bag given; his speed); reads switch 0x1B (Darunia's door open: the
//! lowest floor's texts) and switch 0x1C (the Lost Woods' shortcut open). The adult's (not ported):
//! the Fire Temple Goron's switch flag, INFTABLE_109, _10A, _10B, _10C, _10E, _10F (Link the Goron),
//! INFTABLE_B4 and `bgsFlag` (Biggoron).

use std::cell::Cell;
use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use eng_math::{approach_f, binang_to_rad, cos_s, sin_s, smooth_step_to_s};
use glam::{Mat4, Vec3};
use oot_game::actor::{
    ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_DRAW_CULLING_DISABLED, ACTOR_FLAG_FRIENDLY, ACTOR_FLAG_SFX_FOR_PLAYER_BODY_HIT, ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor, BGCHECKFLAG_GROUND,
    UPDBGCHECKINFO_FLAG_0, UPDBGCHECKINFO_FLAG_2,
};
use oot_game::actor_ctx::{ACTORCAT_NPC, ActorImpl, ActorProfile, actor_set_player_knockback_large_no_damage, audio_play_actor_sfx2, audio_play_actor_sfx_at};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind, Source};
use oot_game::audio::sfx::{NA_SE_EN_DODO_M_GND, NA_SE_EV_BIGBALL_ROLL, NA_SE_PL_BODY_HIT, NA_SE_SY_CORRECT_CHIME, SFX_FLAG, SfxPos};
use oot_game::collision_check::*;
use oot_game::gbi::setup_dl;
use oot_game::get_item::{actor_has_parent, offer_get_item_range};
use oot_game::item::{EQUIP_INV_TUNIC_GORON, EQUIP_TYPE_TUNIC, GI_TUNIC_GORON, ITEM_POCKET_EGG, QUEST_GORON_RUBY, UPG_BOMB_BAG, UPG_STRENGTH};
use oot_game::message::*;
use oot_game::npc::{NpcTrack, actor_update_fidget_tables, offer_talk, process_talk_request, talk_update, track_point};
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::path::{path_copy_last_point, path_get_by_index, path_orient_and_get_dist_sq};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::scene::Path;
use oot_game::skelanime_std::*;
use oot_game::{event_chk_inf, inf_table};

use crate::en_bom::{ACTOR_EN_BOM, EnBom};

/// `ACTOR_EN_GO2` (`actor_table.h`: 0x01AE).
pub const ACTOR_EN_GO2: i16 = 0x01AE;
/// `OBJECT_OF1D_MAP` (`object_table.h`: 0x00C9).
pub const OBJECT: &str = "object_oF1d_map";

/// `En_Go2_Profile`.
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_EN_GO2,
    name: "En_Go2",
    category: ACTORCAT_NPC,
    flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY | ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_DRAW_CULLING_DISABLED,
    object: OBJECT,
    schema: &SCHEMA,
};

// `GoronType` (`z_en_go2.h`).
pub const GORON_CITY_ROLLING_BIG: i16 = 0x00;
pub const GORON_CITY_LINK: i16 = 0x01;
pub const GORON_DMT_BIGGORON: i16 = 0x02;
pub const GORON_FIRE_GENERIC: i16 = 0x03;
pub const GORON_DMT_BOMB_FLOWER: i16 = 0x04;
pub const GORON_DMT_ROLLING_SMALL: i16 = 0x05;
pub const GORON_DMT_DC_ENTRANCE: i16 = 0x06;
pub const GORON_CITY_ENTRANCE: i16 = 0x07;
pub const GORON_CITY_ISLAND: i16 = 0x08;
pub const GORON_CITY_LOWEST_FLOOR: i16 = 0x09;
pub const GORON_CITY_STAIRWELL: i16 = 0x0A;
pub const GORON_CITY_LOST_WOODS: i16 = 0x0B;
pub const GORON_DMT_FAIRY_HINT: i16 = 0x0C;
pub const GORON_MARKET_BAZAAR: i16 = 0x0D;

/// `PARAMS_GET_S(this->actor.params, 0, 5)`: which Goron.
pub const TYPE: Field = Field::params(
    "GoronType",
    0,
    5,
    Kind::Named(&[
        (0x00, "GORON_CITY_ROLLING_BIG: Goron City's big rolling Goron (the 30 bomb bag)"),
        (0x01, "GORON_CITY_LINK: Link the Goron (an adult's; not ported)"),
        (0x02, "GORON_DMT_BIGGORON: Biggoron (an adult's; not ported)"),
        (0x03, "GORON_FIRE_GENERIC: a Goron locked in the Fire Temple (an adult's; not ported)"),
        (0x04, "GORON_DMT_BOMB_FLOWER: the trail's Goron by the cliff's bomb flower"),
        (0x05, "GORON_DMT_ROLLING_SMALL: the trail's rolling Goron (his bomb at the end)"),
        (0x06, "GORON_DMT_DC_ENTRANCE: the Goron by Dodongo's Cavern"),
        (0x07, "GORON_CITY_ENTRANCE: the Goron at Goron City's entrance"),
        (0x08, "GORON_CITY_ISLAND: the Goron on the city's island"),
        (0x09, "GORON_CITY_LOWEST_FLOOR: the Goron by Darunia's door"),
        (0x0A, "GORON_CITY_STAIRWELL: the Goron in the lowest stairwell"),
        (0x0B, "GORON_CITY_LOST_WOODS: the Goron by the Lost Woods' shortcut"),
        (0x0C, "GORON_DMT_FAIRY_HINT: the Goron below the summit (the Great Fairy's hint)"),
        (0x0D, "GORON_MARKET_BAZAAR: the bazaar's Goron (a child with the Goron's Ruby)"),
    ]),
    "which Goron; types 7 to 0xB are killed for an adult without the Fire Medallion, 0xD for an adult or without the Goron's Ruby",
);
/// `PARAMS_GET_S(this->actor.params, 5, 5)`: `Path_GetByIndex(play, .., 0x1F)`.
pub const PATH: Field = Field::params("path", 5, 5, Kind::PathIndex, "the scene path he rolls (types 0, 1, 5) or walks (type 4); 0x1F none");
/// `PARAMS_GET_S(this->actor.params, 10, 6)`: the Fire Temple Goron's switch flag.
pub const SWITCH_FLAG: Field = Field::params(
    "switchFlag",
    10,
    6,
    Kind::SwitchFlag,
    "GORON_FIRE_GENERIC: set, he's been freed (killed at init); his texts; the other types don't read it",
);
/// `this->unk_216 = this->actor.shape.rot.z` (a `u8`): Link the Goron's turn-back waypoint.
pub const UNK_216: Field = Field::rot(
    "unk_216",
    Source::RotZ,
    0,
    8,
    Kind::Range(0, 255),
    "GORON_CITY_LINK: from this waypoint on he turns back when Link is near (EnGo2_IsGoronLinkReversing; not ported)",
);

// `EVENTCHKINF_*`, `INFTABLE_*` (`save.h`).
pub const EVENTCHKINF_23: u16 = 0x23;
pub const INFTABLE_B4: u16 = 0xB4;
pub const INFTABLE_E0: u16 = 0xE0;
pub const INFTABLE_E3: u16 = 0xE3;
pub const INFTABLE_E6: u16 = 0xE6;
pub const INFTABLE_EB: u16 = 0xEB;
pub const INFTABLE_F0: u16 = 0xF0;
pub const INFTABLE_F4: u16 = 0xF4;
pub const INFTABLE_F8: u16 = 0xF8;
pub const INFTABLE_109: u16 = 0x109;
pub const INFTABLE_10A: u16 = 0x10A;
pub const INFTABLE_10B: u16 = 0x10B;
pub const INFTABLE_10C: u16 = 0x10C;
pub const INFTABLE_10E: u16 = 0x10E;
pub const INFTABLE_10F: u16 = 0x10F;
pub const INFTABLE_11E: u16 = 0x11E;

/// En_Go2's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[TYPE, PATH, SWITCH_FLAG, UNK_216],
    flags: &[
        FlagUse::new(event_chk_inf!(EVENTCHKINF_23)).reads().note("the cavern's boulder blown: the trail's rolling and cavern Gorons' texts (0x3026, 0x3021)"),
        FlagUse::new(inf_table!(INFTABLE_E0)).reads().sets().note("the cavern Goron's 0x3008 closed: 0x302A next"),
        FlagUse::new(inf_table!(INFTABLE_E3)).reads().sets().note("the stairwell Goron's 0x300E closed: 0x3022 next"),
        FlagUse::new(inf_table!(INFTABLE_E6)).reads().sets().note("the Lost Woods Goron's 0x3024 closed: 0x3025 next"),
        FlagUse::new(inf_table!(INFTABLE_EB)).reads().sets().note("the bomb-flower Goron's 0x300B closed (with the bracelet): he walks, and is at his path's end from then on"),
        FlagUse::new(inf_table!(INFTABLE_F0)).reads().sets().note("the entrance Goron's 0x3014 closed: 0x3015 next"),
        FlagUse::new(inf_table!(INFTABLE_F4)).reads().sets().note("the island Goron's 0x3016 closed: 0x3017 next"),
        FlagUse::new(inf_table!(INFTABLE_F8)).reads().sets().note("the lowest floor Goron's 0x3018 closed (Darunia's door open): 0x3019 next"),
        FlagUse::new(inf_table!(INFTABLE_11E)).reads().sets().note("the big rolling Goron's bomb bag given: 0x3013, and he starts at 6"),
        FlagUse::new(Flag::Switch("0x1B")).reads().note("Darunia's door open: the lowest floor Goron's texts"),
        FlagUse::new(Flag::Switch("0x1C")).reads().note("the Lost Woods' shortcut open: 0x302F"),
        FlagUse::new(Flag::Switch("switchFlag")).reads().note("GORON_FIRE_GENERIC: set, killed at init (his texts aren't ported)"),
        FlagUse::new(inf_table!(INFTABLE_109)).reads().sets().note("Link the Goron's Goron Tunic given").indirect("not ported (Link the Goron, Phase 15)"),
        FlagUse::new(inf_table!(INFTABLE_10A)).reads().note("Link the Goron's texts").indirect("not ported (Link the Goron, Phase 15)"),
        FlagUse::new(inf_table!(INFTABLE_10B)).reads().sets().note("Link the Goron told about Volvagia").indirect("not ported (Link the Goron, Phase 15)"),
        FlagUse::new(inf_table!(INFTABLE_10C)).reads().sets().clears().note("Link the Goron stopped").indirect("not ported (Link the Goron, Phase 15)"),
        FlagUse::new(inf_table!(INFTABLE_10E)).reads().sets().note("Link the Goron spoken to with the tunic").indirect("not ported (Link the Goron, Phase 15)"),
        FlagUse::new(inf_table!(INFTABLE_10F)).reads().note("Link the Goron's texts with the Fire Medallion").indirect("not ported (Link the Goron, Phase 15)"),
        FlagUse::new(inf_table!(INFTABLE_B4)).reads().note("Biggoron's texts (the broken sword shown)").indirect("not ported (Biggoron, Phase 15)"),
        FlagUse::new(Flag::Other("gSaveContext.save.info.playerData.bgsFlag")).reads().sets().note("Biggoron's Sword given").indirect("not ported (Biggoron, Phase 15)"),
    ],
    todo: false,
    objects: &[],
};

/// `QUEST_MEDALLION_FIRE` (`item.h`: 0x01).
const QUEST_MEDALLION_FIRE: u32 = 0x01;
// `GetItemID` (`item.h`).
pub const GI_CLAIM_CHECK: i16 = 0x26;
pub const GI_BOMB_BAG_30: i16 = 0x33;
pub const GI_BOMB_BAG_40: i16 = 0x34;
pub const GI_SWORD_BIGGORON: i16 = 0x57;
/// `ITEM_TRADE_ADULT` (`item.h`: `ITEM_POCKET_EGG`), `ITEM_BROKEN_GORONS_SWORD` (0x33),
/// `ITEM_EYE_DROPS` (0x36).
const ITEM_TRADE_ADULT: u8 = ITEM_POCKET_EGG;
const ITEM_BROKEN_GORONS_SWORD: u8 = 0x33;
const ITEM_EYE_DROPS: u8 = 0x36;

// Sound effects (`include/tables/sfx/enemybank_table.h`), besides `oot_game::audio::sfx`'s.
const NA_SE_EN_GOLON_LAND_BIG: u16 = 0x3879;
const NA_SE_EN_GOLON_WAKE_UP: u16 = 0x38FC;
const NA_SE_EN_GOLON_SIT_DOWN: u16 = 0x38FD;

/// `NpcTalkState` (`actor.h`).
pub const NPC_TALK_STATE_IDLE: i16 = 0;
pub const NPC_TALK_STATE_TALKING: i16 = 1;
pub const NPC_TALK_STATE_ACTION: i16 = 2;
/// `NpcTrackingMode` (`actor.h`).
pub const NPC_TRACKING_NONE: u16 = 1;
pub const NPC_TRACKING_HEAD_AND_TORSO: u16 = 2;
pub const NPC_TRACKING_FULL_BODY: u16 = 4;
/// `FIDGET_AMPLITUDE` (`actor.h`).
const FIDGET_AMPLITUDE: f32 = 200.0;

/// The joint tables' size (`jointTable[18]`: the skeleton's 17 limbs and the root).
pub const GORON_LIMB_MAX: usize = 18;
/// `EnGo2_OverrideLimbDraw`'s limbs (1-based): the head, the chest, the swaying ones.
const LIMB_HEAD: usize = 17;
const LIMB_CHEST: usize = 10;
const SWAY_LIMBS: [usize; 3] = [10, 11, 14];

/// `EN_GO2_EFFECT_COUNT`.
pub const EN_GO2_EFFECT_COUNT: usize = 10;

/// `sCylinderInit`: takes `DMG_EXPLOSIVE` (0x08) once `EnGo2_SetupRolling` turns it on.
const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_NONE, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_2, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0x0000_0008, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_NONE,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 40, height: 65, y_shift: 0, pos: [0; 3] },
};

/// `sColChkInfoInit`.
const COL_CHK_INFO_INIT: CollisionCheckInfoInit2 = CollisionCheckInfoInit2 { health: 0, cyl_radius: 0, cyl_height: 0, cyl_y_shift: 0, mass: MASS_IMMOVABLE };

/// `EnGo2DataStruct1`: the collider's place and size by type.
#[derive(Debug, Clone, Copy)]
pub struct DataStruct1 {
    pub unused: i16,
    pub y_dist: i16,
    pub xz_dist: i16,
    pub radius: i16,
    pub height: i16,
}

const fn d1(unused: i16, y_dist: i16, xz_dist: i16, radius: i16, height: i16) -> DataStruct1 {
    DataStruct1 { unused, y_dist, xz_dist, radius, height }
}

/// `D_80A4816C`.
pub const D_80A4816C: [DataStruct1; 14] = [
    d1(0, 0, 0, 68, 148),
    d1(0, 0, 0, 24, 52),
    d1(0, 320, 380, 400, 120),
    d1(0, 0, 0, 30, 68),
    d1(0, 0, 0, 46, 90),
    d1(0, 0, 0, 30, 68),
    d1(0, 0, 0, 30, 68),
    d1(0, 0, 0, 30, 68),
    d1(0, 0, 0, 30, 68),
    d1(0, 0, 0, 30, 68),
    d1(0, 0, 0, 30, 68),
    d1(0, 0, 0, 30, 68),
    d1(0, 0, 0, 30, 68),
    d1(0, 0, 0, 30, 68),
];

/// `EnGo2DataStruct2`: the shadow, the scale, the attention range and the talk range by type.
#[derive(Debug, Clone, Copy)]
pub struct DataStruct2 {
    pub shape_unk_10: f32,
    pub scale: f32,
    pub actor_unk_1f: i8,
    pub interact_range: f32,
}

const fn d2(shape_unk_10: f32, scale: f32, actor_unk_1f: i8, interact_range: f32) -> DataStruct2 {
    DataStruct2 { shape_unk_10, scale, actor_unk_1f, interact_range }
}

/// `D_80A481F8`.
pub const D_80A481F8: [DataStruct2; 14] = [
    d2(30.0, 0.026, 6, 60.0),
    d2(24.0, 0.008, 6, 30.0),
    d2(28.0, 0.16, 5, 380.0),
    d2(28.0, 0.01, 7, 40.0),
    d2(30.0, 0.015, 6, 30.0),
    d2(28.0, 0.01, 6, 30.0),
    d2(28.0, 0.01, 6, 30.0),
    d2(28.0, 0.01, 6, 30.0),
    d2(28.0, 0.01, 6, 30.0),
    d2(28.0, 0.01, 6, 30.0),
    d2(28.0, 0.01, 6, 30.0),
    d2(28.0, 0.01, 6, 30.0),
    d2(28.0, 0.01, 6, 30.0),
    d2(28.0, 0.01, 6, 30.0),
];

/// `sPlayerTrackingYOffsets`, by type and `linkAge` (adult first).
pub const PLAYER_TRACKING_Y_OFFSETS: [[f32; 2]; 14] = [
    [80.0, 80.0],
    [-10.0, -10.0],
    [800.0, 800.0],
    [0.0, 0.0],
    [20.0, 40.0],
    [20.0, 20.0],
    [20.0, 20.0],
    [20.0, 20.0],
    [20.0, 20.0],
    [20.0, 20.0],
    [20.0, 20.0],
    [20.0, 20.0],
    [20.0, 20.0],
    [20.0, 20.0],
];

// `EnGo2Animation`.
pub const ENGO2_ANIM_0: usize = 0;
pub const ENGO2_ANIM_1: usize = 1;
pub const ENGO2_ANIM_2: usize = 2;
pub const ENGO2_ANIM_3: usize = 3;
pub const ENGO2_ANIM_4: usize = 4;
pub const ENGO2_ANIM_5: usize = 5;
pub const ENGO2_ANIM_6: usize = 6;
pub const ENGO2_ANIM_7: usize = 7;
pub const ENGO2_ANIM_8: usize = 8;
pub const ENGO2_ANIM_9: usize = 9;
pub const ENGO2_ANIM_10: usize = 10;
pub const ENGO2_ANIM_11: usize = 11;
pub const ENGO2_ANIM_12: usize = 12;

/// `gGoronAnim_004930`: uncurling (forwards) and curling (backwards); the one `func_80A45360` and
/// `EnGo2_SitDownAnimation` look for.
pub const ANIM_004930: &str = "gGoronAnim_004930";

/// `sAnimationInfo`: animation, speed, start frame, frame count, mode, morph frames.
const ANIMATION_INFO: [(&str, f32, f32, f32, u8, f32); 13] = [
    (ANIM_004930, 0.0, 0.0, -1.0, ANIMMODE_LOOP, 0.0),
    (ANIM_004930, 0.0, 0.0, -1.0, ANIMMODE_LOOP, -8.0),
    ("gGoronAnim_0029A8", 1.0, 0.0, -1.0, ANIMMODE_LOOP, -8.0),
    ("gGoronAnim_010590", 1.0, 0.0, -1.0, ANIMMODE_LOOP, -8.0),
    ("gGoronAnim_003768", 1.0, 0.0, -1.0, ANIMMODE_LOOP, -8.0),
    ("gGoronAnim_0038E4", 1.0, 0.0, -1.0, ANIMMODE_ONCE, -8.0),
    ("gGoronAnim_002D80", 1.0, 0.0, -1.0, ANIMMODE_ONCE, -8.0),
    ("gGoronAnim_00161C", 1.0, 0.0, -1.0, ANIMMODE_LOOP, -8.0),
    ("gGoronAnim_001A00", 1.0, 0.0, -1.0, ANIMMODE_LOOP, -8.0),
    ("gGoronAnim_0021D0", 1.0, 0.0, -1.0, ANIMMODE_LOOP, -8.0),
    (ANIM_004930, 0.0, 0.0, -1.0, ANIMMODE_LOOP_INTERP, -8.0),
    ("gGoronAnim_000750", 1.0, 0.0, -1.0, ANIMMODE_LOOP, -8.0),
    ("gGoronAnim_000D5C", 1.0, 0.0, -1.0, ANIMMODE_LOOP, -8.0),
];

/// `EnGo2DustEffectData`.
#[derive(Debug, Clone, Copy)]
pub struct DustEffectData {
    pub initial_timer: u8,
    pub scale: f32,
    pub scale_step: f32,
    pub num_dust_effects: i32,
    pub radius: f32,
    pub y_accel: f32,
}

const fn dust(initial_timer: u8, scale: f32, scale_step: f32, num_dust_effects: i32, radius: f32, y_accel: f32) -> DustEffectData {
    DustEffectData { initial_timer, scale, scale_step, num_dust_effects, radius, y_accel }
}

/// `sDustEffectData`: the others' and the big rolling Goron's, by occasion (0 a bounce, 1 curled
/// up or settled, 2 spinning in place, 3 a roll's bounce).
pub const DUST_EFFECT_DATA: [[DustEffectData; 4]; 2] = [
    [dust(12, 0.2, 0.2, 1, 18.0, 0.0), dust(12, 0.1, 0.2, 12, 26.0, 0.0), dust(12, 0.1, 0.3, 4, 10.0, 0.0), dust(12, 0.2, 0.2, 1, 18.0, 0.0)],
    [dust(12, 0.5, 0.4, 3, 42.0, 0.0), dust(12, 0.5, 0.4, 3, 42.0, 0.0), dust(12, 0.5, 0.4, 3, 42.0, 0.0), dust(12, 0.5, 0.4, 3, 42.0, 0.0)],
];

/// `sPos`, `sVelocity`, `sAccel`: `EnGo2_SpawnDust`'s starting values.
const S_VELOCITY: Vec3 = Vec3::ZERO;
const S_ACCEL: Vec3 = Vec3::new(0.0, 0.3, 0.0);

/// `sDustTex`, by `EnGo2_DrawEffects`' index.
const DUST_TEXTURES: [&str; 8] = ["gDust8Tex", "gDust7Tex", "gDust6Tex", "gDust5Tex", "gDust4Tex", "gDust3Tex", "gDust2Tex", "gDust1Tex"];
/// `EnGo2_Draw`'s `eyeTextures` (segment 8) and `mouthTextures` (segment 9).
const EYE_TEXTURES: [&str; 4] = ["gGoronCsEyeClosed2Tex", "gGoronCsEyeOpenTex", "gGoronCsEyeHalfTex", "gGoronCsEyeClosedTex"];
const MOUTH_TEXTURES: [&str; 2] = ["gGoronCsMouthNeutralTex", "gGoronCsMouthSmileTex"];
const SEG_EYES: u8 = 0x08;
const SEG_MOUTH: u8 = 0x09;
/// The dust's segments: the texture (8, as `gGoronDL_00FD50` loads it), its setup and colours.
const SEG_DUST_TEX: u8 = 0x08;
const SEG_DUST_SETUP: u8 = 0x0A;
const SEG_DUST_COLOR: u8 = 0x0B;
/// `gDPSetEnvColor(100, 60, 20, 0)`, `gDPSetPrimColor(0, 0, 170, 130, 90, alpha)`.
const DUST_ENV: [u8; 4] = [100, 60, 20, 0];
const DUST_PRIM: [u8; 3] = [170, 130, 90];
/// `gGoronDL_00BD80` (curled up), `gGoronDL_00C140` (rolling).
pub const CURLED_DL: &str = "gGoronDL_00BD80";
pub const ROLLING_DL: &str = "gGoronDL_00C140";

/// The skeleton drawn with `eyeTextures[eye]` and `mouthTextures[mouth]`.
pub fn bake_name(eye: usize, mouth: usize) -> String {
    format!("En_Go2/eye{eye}_mouth{mouth}")
}

/// A puff of dust drawn with `sDustTex[index]`.
pub fn dust_bake_name(index: usize) -> String {
    format!("En_Go2/dust{index}")
}

/// The meshes `EnGo2_Draw` draws: the skeleton with each eye and mouth, and `EnGo2_DrawEffects`'
/// puff with each dust texture (`Gfx_SetupDL_25Xlu`, then `SETUPDL_0`, whose modes replace it;
/// `gGoronDL_00FD40`'s material, its colours, `gGoronDL_00FD50`).
pub fn bakes() -> Vec<MeshBake> {
    let mut v = Vec::new();
    for (eye, eye_tex) in EYE_TEXTURES.iter().enumerate() {
        for (mouth, mouth_tex) in MOUTH_TEXTURES.iter().enumerate() {
            v.push(MeshBake {
                name: bake_name(eye, mouth),
                object: OBJECT.into(),
                segments: vec![
                    (SEG_EYES, BakeSegment::Texture { file: OBJECT.into(), symbol: (*eye_tex).into() }),
                    (SEG_MOUTH, BakeSegment::Texture { file: OBJECT.into(), symbol: (*mouth_tex).into() }),
                ],
                prelude: Vec::new(),
                body: BakeBody::Skeleton { file: OBJECT.into(), symbol: "gGoronSkel".into(), limbs: Vec::new() },
            });
        }
    }
    let mut setup = setup_dl::setup_dl_0();
    setup.end();
    for (index, tex) in DUST_TEXTURES.iter().enumerate() {
        v.push(MeshBake {
            name: dust_bake_name(index),
            object: OBJECT.into(),
            segments: vec![
                (SEG_DUST_TEX, BakeSegment::Texture { file: "gameplay_keep".into(), symbol: (*tex).into() }),
                (SEG_DUST_SETUP, BakeSegment::Commands(setup.0.clone())),
                (SEG_DUST_COLOR, BakeSegment::DynamicColor { env: true, prim: true }),
            ],
            prelude: vec![SEG_DUST_SETUP, SEG_DUST_COLOR],
            body: BakeBody::DLists(vec![(OBJECT.into(), "gGoronDL_00FD40".into()), (OBJECT.into(), "gGoronDL_00FD50".into())]),
        });
    }
    v
}

/// `EnGoEffect` (`z_en_go.h`): a puff of dust (`type` 1 while it lives).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct EnGoEffect {
    pub type_: u8,
    pub timer: u8,
    pub initial_timer: u8,
    pub scale: f32,
    pub scale_step: f32,
    pub color: [u8; 4],
    pub pos: Vec3,
    pub velocity: Vec3,
    pub accel: Vec3,
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `EnGo2_CurledUp`.
    CurledUp,
    /// `func_80A46B40`: uncurling, then awake (sitting or standing, talking).
    Func80A46B40,
    /// `EnGo2_GoronDmtBombFlowerAnimation`: the bomb-flower Goron's walk.
    GoronDmtBombFlowerAnimation,
    /// `EnGo2_GoronRollingBigContinueRolling`: the big rolling Goron curling to roll on.
    GoronRollingBigContinueRolling,
    /// `EnGo2_ContinueRolling`: spinning in place.
    ContinueRolling,
    /// `EnGo2_SlowRolling`: rolling the path.
    SlowRolling,
    /// `EnGo2_GroundRolling`: settling after a stop.
    GroundRolling,
    /// `EnGo2_ReverseRolling` (Link the Goron's; not ported).
    ReverseRolling,
    /// `EnGo2_SetupGetItem`: offering the item.
    SetupGetItem,
    /// `EnGo2_SetGetItem`: the item's text.
    SetGetItem,
    /// `EnGo2_BiggoronEyedrops` (not ported).
    BiggoronEyedrops,
    /// `EnGo2_GoronLinkStopRolling` (not ported).
    GoronLinkStopRolling,
    /// `EnGo2_GoronFireGenericAction` (not ported).
    GoronFireGenericAction,
}

/// What the talk callbacks read and write besides the actor.
#[derive(Debug, Clone, Copy)]
struct TalkCtx {
    ty: i16,
    waypoint: i8,
    action: Action,
    get_item_id: i32,
}

pub struct EnGo2 {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    pub skeleton: Arc<Skeleton>,
    pub action: Action,
    pub interact_info: NpcTrack,
    pub collider: ColliderCylinder,
    /// `path`: the scene path's index (`Path_GetByIndex`).
    pub path: Option<usize>,
    /// `unk_20C`: Link the Goron's animation's counter.
    pub unk_20c: u8,
    pub dialog_state: u8,
    pub reverse: u8,
    pub is_awake: bool,
    pub waypoint: i8,
    pub unk_211: bool,
    pub goron_state: u8,
    pub eye_mouth_tex_state: u8,
    pub eye_tex_index: u8,
    pub mouth_tex_index: u8,
    pub unk_216: u8,
    pub interact_range: f32,
    /// `alpha`: smoothed to `shape.shadowAlpha` (`func_80A45360`).
    pub alpha: f32,
    pub blink_timer: i16,
    pub fidget_table_y: [i16; GORON_LIMB_MAX],
    pub fidget_table_z: [i16; GORON_LIMB_MAX],
    pub tracking_mode: u16,
    pub effects: [EnGoEffect; EN_GO2_EFFECT_COUNT],
    pub sub_cam_eye: Vec3,
    pub sub_cam_at: Vec3,
    /// `unk_590`: the rolling's timer (`EnGo2_IsRollingOnGround`).
    pub unk_590: i16,
    pub anim_timer: i16,
    pub get_item_id: i32,
    pub sub_cam_id: i16,
    /// `unk_59C`: the bounces left (`EnGo2_IsRollingOnGround`).
    pub unk_59c: i16,
    /// `EnGo2_DrawRolling`'s spin (`play->state.frames * ((s16)speedXZ * 1400)`), at the last draw.
    pub roll_angle: i16,
    animations: Vec<AnimationInfo>,
}

/// `DECR(x)`: down to 0, the new value.
fn decr(x: &mut i16) -> i16 {
    if *x != 0 {
        *x -= 1;
    }
    *x
}

/// `CHECK_QUEST_ITEM(QUEST_MEDALLION_FIRE) && LINK_IS_ADULT`: the adult's text 0x3043.
fn adult_with_fire_medallion(play: &PlayState) -> bool {
    play.save.check_quest_item(QUEST_MEDALLION_FIRE) && play.save.adult
}

/// `EnGo2_GetTextId` (`Npc_UpdateTalking`'s first callback): by type.
fn get_text_id(play: &PlayState, ctx: &TalkCtx) -> u16 {
    // MaskReaction_GetTextId(play, MASK_REACTION_SET_GORON): no masks, so 0.
    let s = &play.save;
    match ctx.ty {
        // EnGo2_GetTextIdGoronCityRollingBig.
        GORON_CITY_ROLLING_BIG => {
            if s.get_inf_table(INFTABLE_11E) {
                0x3013
            } else if s.cur_capacity(UPG_BOMB_BAG) >= 20 && ctx.waypoint > 7 && ctx.waypoint < 12 {
                0x3012
            } else {
                0x3011
            }
        }
        GORON_CITY_LINK | GORON_DMT_BIGGORON | GORON_FIRE_GENERIC => {
            log::debug!("En_Go2: the texts of type {} (EnGo2_GetTextIdGoronCityLink, _GoronDmtBiggoron, _GoronFireGeneric; Phase 15) aren't ported", ctx.ty);
            0
        }
        // EnGo2_GetTextIdGoronDmtBombFlower.
        GORON_DMT_BOMB_FLOWER => {
            if s.check_quest_item(QUEST_GORON_RUBY) {
                0x3027
            } else {
                0x300A
            }
        }
        // EnGo2_GetTextIdGoronDmtRollingSmall.
        GORON_DMT_ROLLING_SMALL => {
            if s.check_quest_item(QUEST_GORON_RUBY) {
                0x3027
            } else if s.get_event_chk_inf(EVENTCHKINF_23) {
                0x3026
            } else {
                0x3009
            }
        }
        // EnGo2_GetTextIdGoronDmtDcEntrance.
        GORON_DMT_DC_ENTRANCE => {
            if adult_with_fire_medallion(play) {
                0x3043
            } else if s.check_quest_item(QUEST_GORON_RUBY) {
                0x3027
            } else if s.get_event_chk_inf(EVENTCHKINF_23) {
                0x3021
            } else if s.get_inf_table(INFTABLE_E0) {
                0x302A
            } else {
                0x3008
            }
        }
        // EnGo2_GetTextIdGoronCityEntrance.
        GORON_CITY_ENTRANCE => {
            if adult_with_fire_medallion(play) {
                0x3043
            } else if s.check_quest_item(QUEST_GORON_RUBY) {
                0x3027
            } else if s.get_inf_table(INFTABLE_F0) {
                0x3015
            } else {
                0x3014
            }
        }
        // EnGo2_GetTextIdGoronCityIsland.
        GORON_CITY_ISLAND => {
            if adult_with_fire_medallion(play) {
                0x3043
            } else if s.check_quest_item(QUEST_GORON_RUBY) {
                0x3067
            } else if s.get_inf_table(INFTABLE_F4) {
                0x3017
            } else {
                0x3016
            }
        }
        // EnGo2_GetTextIdGoronCityLowestFloor.
        GORON_CITY_LOWEST_FLOOR => {
            if adult_with_fire_medallion(play) {
                0x3043
            } else if s.check_quest_item(QUEST_GORON_RUBY) {
                0x3027
            } else if s.cur_upg_value(UPG_STRENGTH) != 0 {
                0x302C
            } else if !play.flags.get_switch(0x1B) {
                0x3017
            } else if s.get_inf_table(INFTABLE_F8) {
                0x3019
            } else {
                0x3018
            }
        }
        // EnGo2_GetTextIdGoronCityStairwell.
        GORON_CITY_STAIRWELL => {
            if !s.adult {
                if s.get_inf_table(INFTABLE_E3) { 0x3022 } else { 0x300E }
            } else {
                0x3043
            }
        }
        // EnGo2_GetTextIdGoronCityLostWoods.
        GORON_CITY_LOST_WOODS => {
            if !s.adult {
                if play.flags.get_switch(0x1C) {
                    0x302F
                } else if s.get_inf_table(INFTABLE_E6) {
                    0x3025
                } else {
                    0x3024
                }
            } else {
                0x3043
            }
        }
        // EnGo2_GetTextIdGoronDmtFairyHint.
        GORON_DMT_FAIRY_HINT => {
            if !s.adult {
                if s.check_quest_item(QUEST_GORON_RUBY) { 0x3065 } else { 0x3064 }
            } else {
                0x3043
            }
        }
        // EnGo2_GetTextIdGoronMarketBazaar.
        GORON_MARKET_BAZAAR => 0x7122,
        // (AVOID_UB: the mask reaction's 0.)
        _ => 0,
    }
}

/// `Message_CloseTextbox`.
fn close_textbox(play: &mut PlayState) {
    play.with_msg(|m, f| m.close_textbox(f.audio));
}

/// `EnGo2_GetItem`: `getItemId` recorded and offered (`Actor_OfferGetItem` within Link's distance
/// plus 1).
fn get_item(play: &mut PlayState, actor: &Actor, get_item_id: &mut i32, gi: i16) {
    *get_item_id = gi as i32;
    offer_get_item_range(play, actor, gi, actor.xz_dist_to_player + 1.0, actor.y_dist_to_player.abs() + 1.0);
}

/// The talk states of the texts that only close: `NPC_TALK_STATE_IDLE` as the box closes, else
/// `NPC_TALK_STATE_TALKING`; true in `.1` when it's closing (the flag of the text set by the caller).
fn on_closing(play: &PlayState) -> (i16, bool) {
    if play.message_state() == TEXT_STATE_CLOSING { (NPC_TALK_STATE_IDLE, true) } else { (NPC_TALK_STATE_TALKING, false) }
}

/// `EnGo2_UpdateTalkState` (`Npc_UpdateTalking`'s second callback): by type.
fn update_talk_state(play: &mut PlayState, actor: &mut Actor, ctx: &mut TalkCtx) -> i16 {
    match ctx.ty {
        // EnGo2_UpdateTalkStateGoronCityRollingBig.
        GORON_CITY_ROLLING_BIG => match play.message_state() {
            TEXT_STATE_CLOSING => NPC_TALK_STATE_ACTION,
            TEXT_STATE_EVENT if play.message_should_advance() => {
                if actor.text_id == 0x3012 {
                    ctx.action = Action::SetupGetItem;
                    let bomb_bag_upgrade = if play.save.cur_capacity(UPG_BOMB_BAG) == 30 { GI_BOMB_BAG_40 } else { GI_BOMB_BAG_30 };
                    get_item(play, actor, &mut ctx.get_item_id, bomb_bag_upgrade);
                    close_textbox(play);
                    play.save.set_inf_table(INFTABLE_11E);
                }
                NPC_TALK_STATE_ACTION
            }
            _ => NPC_TALK_STATE_TALKING,
        },
        GORON_CITY_LINK | GORON_DMT_BIGGORON | GORON_FIRE_GENERIC => {
            log::debug!("En_Go2: the talk of type {} (EnGo2_UpdateTalkStateGoronCityLink, _GoronDmtBiggoron, _GoronFireGeneric; Phase 15) isn't ported", ctx.ty);
            NPC_TALK_STATE_TALKING
        }
        // EnGo2_UpdateTalkStateGoronDmtBombFlower: the choice.
        GORON_DMT_BOMB_FLOWER => match play.message_state() {
            TEXT_STATE_CLOSING => {
                if actor.text_id == 0x300B && !play.save.get_inf_table(INFTABLE_EB) {
                    play.save.set_inf_table(INFTABLE_EB);
                    NPC_TALK_STATE_ACTION
                } else {
                    NPC_TALK_STATE_IDLE
                }
            }
            TEXT_STATE_CHOICE if play.message_should_advance() => {
                // Ask question to DMT Goron by bomb flower.
                if actor.text_id == 0x300A {
                    actor.text_id = if play.msg_ctx.choice_index == 0 {
                        if play.save.cur_upg_value(UPG_STRENGTH) != 0 { 0x300B } else { 0x300C }
                    } else {
                        0x300D
                    };
                    play.continue_textbox(actor.text_id);
                }
                NPC_TALK_STATE_TALKING
            }
            _ => NPC_TALK_STATE_TALKING,
        },
        // EnGo2_UpdateTalkStateGoronDmtRollingSmall, _GoronMarketBazaar, _GoronDmtFairyHint.
        GORON_DMT_ROLLING_SMALL | GORON_MARKET_BAZAAR | GORON_DMT_FAIRY_HINT => on_closing(play).0,
        // EnGo2_UpdateTalkStateGoronDmtDcEntrance.
        GORON_DMT_DC_ENTRANCE => {
            let (state, closing) = on_closing(play);
            if closing && actor.text_id == 0x3008 {
                play.save.set_inf_table(INFTABLE_E0);
            }
            state
        }
        // EnGo2_UpdateTalkStateGoronCityEntrance.
        GORON_CITY_ENTRANCE => {
            let (state, closing) = on_closing(play);
            if closing && actor.text_id == 0x3014 {
                play.save.set_inf_table(INFTABLE_F0);
            }
            state
        }
        // EnGo2_UpdateTalkStateGoronCityIsland.
        GORON_CITY_ISLAND => {
            let (state, closing) = on_closing(play);
            if closing && actor.text_id == 0x3016 {
                play.save.set_inf_table(INFTABLE_F4);
            }
            state
        }
        // EnGo2_UpdateTalkStateGoronCityLowestFloor.
        GORON_CITY_LOWEST_FLOOR => {
            let (state, closing) = on_closing(play);
            if closing && actor.text_id == 0x3018 {
                play.save.set_inf_table(INFTABLE_F8);
            }
            state
        }
        // EnGo2_UpdateTalkStateGoronCityStairwell.
        GORON_CITY_STAIRWELL => {
            let (state, closing) = on_closing(play);
            if closing && actor.text_id == 0x300E {
                play.save.set_inf_table(INFTABLE_E3);
            }
            state
        }
        // EnGo2_UpdateTalkStateGoronCityLostWoods.
        GORON_CITY_LOST_WOODS => {
            let (state, closing) = on_closing(play);
            if closing && actor.text_id == 0x3024 {
                play.save.set_inf_table(INFTABLE_E6);
            }
            state
        }
        // (AVOID_UB: false.)
        _ => NPC_TALK_STATE_IDLE,
    }
}

impl EnGo2 {
    /// `EnGo2_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let Some(assets) = play.assets.clone() else {
            let mut a = actor;
            a.kill();
            return oot_game::spawn::Placeholder::init(a, play);
        };
        let loaded = assets.skeleton(OBJECT, "gGoronSkel").and_then(|s| {
            let animations = ANIMATION_INFO
                .iter()
                .map(|&(name, play_speed, start_frame, frame_count, mode, morph_frames)| {
                    assets.animation(OBJECT, name).map(|animation| AnimationInfo { animation, play_speed, start_frame, frame_count, mode, morph_frames })
                })
                .collect::<anyhow::Result<Vec<_>>>()?;
            Ok((s, animations))
        });
        let (skeleton, animations) = match loaded {
            Ok(x) => x,
            Err(e) => {
                log::error!("En_Go2: {e:#}");
                let mut a = actor;
                a.kill();
                return oot_game::spawn::Placeholder::init(a, play);
            }
        };
        let ty = TYPE.get(actor.params);
        if ty > GORON_MARKET_BAZAAR {
            // The C reads past D_80A4816C, D_80A481F8 and sPlayerTrackingYOffsets: no placement does.
            log::warn!("En_Go2: type {ty:#x} reads past the data tables: killed");
            let mut a = actor;
            a.kill();
            return oot_game::spawn::Placeholder::init(a, play);
        }
        let mut this = EnGo2 {
            actor,
            // SkelAnime_InitFlex(.., &gGoronSkel, NULL, jointTable, morphTable, 18).
            skel: SkelAnimeStd::init_flex(skeleton.limbs.len(), None),
            skeleton,
            action: Action::CurledUp,
            interact_info: NpcTrack::default(),
            collider: ColliderCylinder::new(&CYLINDER_INIT),
            path: None,
            unk_20c: 0,
            dialog_state: 0,
            reverse: 0,
            is_awake: false,
            waypoint: 0,
            unk_211: false,
            goron_state: 0,
            eye_mouth_tex_state: 0,
            eye_tex_index: 0,
            mouth_tex_index: 0,
            unk_216: 0,
            interact_range: 0.0,
            alpha: 0.0,
            blink_timer: 0,
            fidget_table_y: [0; GORON_LIMB_MAX],
            fidget_table_z: [0; GORON_LIMB_MAX],
            tracking_mode: NPC_TRACKING_NONE,
            effects: [EnGoEffect::default(); EN_GO2_EFFECT_COUNT],
            sub_cam_eye: Vec3::ZERO,
            sub_cam_at: Vec3::ZERO,
            unk_590: 0,
            anim_timer: 0,
            get_item_id: 0,
            sub_cam_id: 0,
            unk_59c: 0,
            roll_angle: 0,
            animations,
        };
        this.actor.shape_init(0.0, oot_game::shadow::ShadowDraw::Circle, 28.0);
        // Collider_InitCylinder, Collider_SetCylinder: above.
        this.actor.col_chk_info.set_info2(None, &COL_CHK_INFO_INIT);

        // Not GORON_CITY_ROLLING_BIG, GORON_CITY_LINK, GORON_DMT_BIGGORON.
        if (GORON_FIRE_GENERIC..=GORON_MARKET_BAZAAR).contains(&ty) {
            this.actor.flags &= !ACTOR_FLAG_UPDATE_CULLING_DISABLED;
            this.actor.flags &= !ACTOR_FLAG_DRAW_CULLING_DISABLED;
        }

        this.set_collider_dim();
        this.set_shape();
        this.change_anim(ENGO2_ANIM_0);
        this.actor.gravity = -1.0;
        this.alpha = 0.0;
        this.actor.shape_shadow_alpha = 0;
        this.reverse = 0;
        this.is_awake = false;
        this.unk_211 = false;
        this.goron_state = 0;
        this.waypoint = 0;
        this.unk_216 = UNK_216.get(this.actor.shape_rot.z) as u8;
        this.tracking_mode = NPC_TRACKING_NONE;
        this.path = path_get_by_index(play.setup_path_list(), PATH.get(this.actor.params), 0x1F);
        let s = &play.save;
        match ty {
            GORON_CITY_ENTRANCE | GORON_CITY_ISLAND | GORON_CITY_LOWEST_FLOOR | GORON_CITY_STAIRWELL | GORON_CITY_LOST_WOODS => {
                if !s.check_quest_item(QUEST_MEDALLION_FIRE) && s.adult {
                    this.actor.kill();
                }
                this.action = Action::CurledUp;
            }
            GORON_MARKET_BAZAAR => {
                if s.adult || !s.check_quest_item(QUEST_GORON_RUBY) {
                    this.actor.kill();
                }
                this.get_item_animation();
            }
            GORON_CITY_LINK => {
                log::info!("En_Go2: Link the Goron (GORON_CITY_LINK: his rolling, his talk and the Goron Tunic, Phase 15) isn't ported: a placeholder");
                return oot_game::spawn::Placeholder::init(this.actor, play);
            }
            GORON_CITY_ROLLING_BIG | GORON_DMT_ROLLING_SMALL => {
                this.collider.dim.height = (D_80A4816C[ty as usize].height as f32 * 0.6) as i16;
                this.setup_rolling(play);
            }
            GORON_FIRE_GENERIC => {
                if play.flags.get_switch(SWITCH_FLAG.get(this.actor.params) as i32) {
                    this.actor.kill();
                    return Box::new(this);
                }
                log::info!("En_Go2: the Fire Temple's Goron (GORON_FIRE_GENERIC: his freeing, his texts and his walk out, Phase 15) isn't ported: a placeholder");
                return oot_game::spawn::Placeholder::init(this.actor, play);
            }
            GORON_DMT_BIGGORON => {
                this.actor.shape_shadow_draw = oot_game::shadow::ShadowDraw::None;
                log::info!("En_Go2: Biggoron (GORON_DMT_BIGGORON: his texts, the eye drops and his sword, Phase 15) isn't ported: a placeholder");
                return oot_game::spawn::Placeholder::init(this.actor, play);
            }
            GORON_DMT_BOMB_FLOWER => {
                if s.get_inf_table(INFTABLE_EB) {
                    let path = this.path_of(play);
                    path_copy_last_point(path, &mut this.actor.world_pos);
                    this.actor.home_pos = this.actor.world_pos;
                }
                this.action = Action::CurledUp;
            }
            // GORON_DMT_DC_ENTRANCE, GORON_DMT_FAIRY_HINT.
            _ => this.action = Action::CurledUp,
        }
        Box::new(this)
    }

    /// `PARAMS_GET_S(this->actor.params, 0, 5)`.
    fn ty(&self) -> i16 {
        TYPE.get(self.actor.params)
    }

    /// The scene path `this->path` points to.
    fn path_of<'a>(&self, play: &'a PlayState) -> Option<&'a Path> {
        self.path.and_then(|i| play.setup_path_list().get(i))
    }

    /// `Animation_ChangeByInfo(&skelAnime, sAnimationInfo, index)`.
    fn change_anim(&mut self, index: usize) {
        if let Some(info) = self.animations.get(index) {
            self.skel.change_by_info(info);
        }
    }

    /// `Animation_OnFrame(&skelAnime, skelAnime.endFrame)`.
    fn on_end_frame(&self) -> bool {
        self.skel.on_frame(self.skel.end_frame)
    }

    /// `EnGo2_SpawnEffectDust`: the first free puff.
    fn spawn_effect_dust(&mut self, pos: Vec3, velocity: Vec3, accel: Vec3, initial_timer: u8, scale: f32, scale_step: f32) {
        if let Some(e) = self.effects.iter_mut().find(|e| e.type_ != 1) {
            e.scale = scale;
            e.scale_step = scale_step;
            e.timer = initial_timer;
            e.initial_timer = initial_timer;
            e.type_ = 1;
            e.pos = pos;
            e.accel = accel;
            e.velocity = velocity;
        }
    }

    /// `EnGo2_UpdateEffects` (from `EnGo2_Draw`): each puff drifts, grows and runs out.
    fn update_effects(&mut self, play: &mut PlayState) {
        for e in self.effects.iter_mut() {
            if e.type_ == 0 {
                continue;
            }
            e.timer = e.timer.wrapping_sub(1);
            if e.timer == 0 {
                e.type_ = 0;
            }
            e.accel.x = (play.rand.zero_one() * 0.4) - 0.2;
            let random_number = play.rand.zero_one() * 0.4;
            e.accel.z = random_number - 0.2;
            e.pos += e.velocity;
            e.velocity.x += e.accel.x;
            e.velocity.y += e.accel.y;
            e.velocity.z += random_number - 0.2;
            e.scale += e.scale_step;
        }
    }

    /// `EnGo2_SpawnDust`: `numDustEffects + 1` puffs round him at `radius` on the floor, from a
    /// random angle.
    #[allow(clippy::too_many_arguments)]
    fn spawn_dust(&mut self, play: &mut PlayState, initial_timer: u8, scale: f32, scale_step: f32, num_dust_effects: i32, radius: f32, y_accel: f32) -> i32 {
        let velocity = S_VELOCITY;
        let mut accel = S_ACCEL;
        // pos = sPos, overwritten.
        let mut pos = self.actor.world_pos;
        pos.y = self.actor.floor_height;
        let mut angle = ((play.rand.zero_one() - 0.5) * 65536.0) as i32 as i16;
        let mut i = num_dust_effects;
        while i >= 0 {
            accel.y += play.rand.zero_one() * y_accel;
            pos.x = (sin_s(angle) * radius) + self.actor.world_pos.x;
            pos.z = (cos_s(angle) * radius) + self.actor.world_pos.z;
            self.spawn_effect_dust(pos, velocity, accel, initial_timer, scale, scale_step);
            angle = angle.wrapping_add((0x10000 / num_dust_effects) as i16);
            i -= 1;
        }
        0
    }

    /// `EnGo2_GetDustData`: the dust of occasion `index2` (the big rolling Goron's bigger).
    fn get_dust_data(&mut self, play: &mut PlayState, index2: usize) {
        let index1 = if self.ty() == GORON_CITY_ROLLING_BIG { 1 } else { 0 };
        let d = DUST_EFFECT_DATA[index1][index2];
        self.spawn_dust(play, d.initial_timer, d.scale, d.scale_step, d.num_dust_effects, d.radius, d.y_accel);
    }

    fn talk_ctx(&self) -> TalkCtx {
        TalkCtx { ty: self.ty(), waypoint: self.waypoint, action: self.action, get_item_id: self.get_item_id }
    }

    fn set_talk_ctx(&mut self, ctx: TalkCtx) {
        self.action = ctx.action;
        self.get_item_id = ctx.get_item_id;
    }

    /// `func_80A44790`: the talk. `Npc_UpdateTalking` but for Biggoron (only while Link touches
    /// him) and the big rolling Goron (`Actor_OfferTalk`, wherever he's seen from). True when a talk
    /// starts.
    fn func_80a44790(&mut self, play: &mut PlayState) -> bool {
        let ty = self.ty();
        if ty != GORON_DMT_BIGGORON && ty != GORON_CITY_ROLLING_BIG {
            let ctx = Cell::new(self.talk_ctx());
            let mut talk_state = self.interact_info.talk_state;
            let started = talk_update(
                play,
                &mut self.actor,
                &mut talk_state,
                self.interact_range,
                |play, _| get_text_id(play, &ctx.get()),
                |play, actor| {
                    let mut c = ctx.get();
                    let r = update_talk_state(play, actor, &mut c);
                    ctx.set(c);
                    r
                },
            );
            self.interact_info.talk_state = talk_state;
            self.set_talk_ctx(ctx.get());
            started
        } else if ty == GORON_DMT_BIGGORON && self.collider.base.oc_flags2 & OC2_HIT_PLAYER == 0 {
            false
        } else {
            if process_talk_request(&mut self.actor) {
                self.interact_info.talk_state = NPC_TALK_STATE_TALKING;
                return true;
            } else if self.interact_info.talk_state != NPC_TALK_STATE_IDLE {
                let mut c = self.talk_ctx();
                self.interact_info.talk_state = update_talk_state(play, &mut self.actor, &mut c);
                self.set_talk_ctx(c);
                return false;
            } else if offer_talk(play, &self.actor, self.interact_range) {
                self.actor.text_id = get_text_id(play, &self.talk_ctx());
            }
            false
        }
    }

    /// `EnGo2_SetColliderDim`.
    fn set_collider_dim(&mut self) {
        let index = self.ty() as usize;
        self.collider.dim.radius = D_80A4816C[index].radius;
        self.collider.dim.height = D_80A4816C[index].height;
    }

    /// `EnGo2_SetShape`.
    fn set_shape(&mut self) {
        let index = self.ty() as usize;
        self.actor.shape_shadow_scale = D_80A481F8[index].shape_unk_10;
        // Actor_SetScale.
        self.actor.scale = Vec3::splat(D_80A481F8[index].scale);
        self.actor.target_mode = D_80A481F8[index].actor_unk_1f as u8;
        self.interact_range = D_80A481F8[index].interact_range;
        self.interact_range += self.collider.dim.radius as f32;
    }

    /// `EnGo2_CheckCollision`: the cylinder (out in front for Biggoron), OC and AC.
    fn check_collision(&mut self, play: &mut PlayState) {
        let mut pos = [self.actor.world_pos.x as i32 as i16, self.actor.world_pos.y as i32 as i16, self.actor.world_pos.z as i32 as i16];
        let d = D_80A4816C[self.ty() as usize];
        let xz_dist = d.xz_dist as f32;
        pos[0] = pos[0].wrapping_add((xz_dist * sin_s(self.actor.shape_rot.y)) as i32 as i16);
        pos[2] = pos[2].wrapping_add((xz_dist * cos_s(self.actor.shape_rot.y)) as i32 as i16);
        pos[1] = pos[1].wrapping_add(d.y_dist);
        self.collider.dim.pos = pos;
        play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
        play.collision_check_set_ac(&self.actor, 0, &mut self.collider);
    }

    /// `EnGo2_SwapInitialFrameAnimFrameCount`.
    fn swap_initial_frame_anim_frame_count(&mut self) {
        let initial_frame = self.skel.start_frame;
        self.skel.start_frame = self.skel.end_frame;
        self.skel.end_frame = initial_frame;
    }

    /// `func_80A44AB0`: while rolling, a hit (the big one's `DMG_EXPLOSIVE`) stops him, and
    /// touching Link knocks him down and costs 4.
    fn func_80a44ab0(&mut self, play: &mut PlayState) -> bool {
        if self.ty() == GORON_DMT_BIGGORON {
            return false;
        }
        if !matches!(self.action, Action::SlowRolling | Action::ReverseRolling | Action::ContinueRolling) {
            return false;
        }
        if self.collider.base.ac_flags & AC_HIT != 0 {
            play.audio.play_sfx_centered(NA_SE_SY_CORRECT_CHIME);
            self.actor.flags &= !ACTOR_FLAG_SFX_FOR_PLAYER_BODY_HIT;
            self.collider.base.ac_flags &= !AC_HIT;
            self.stop_rolling(play);
            return true;
        }
        let invincibility_timer = play.player.and_then(|h| play.actors.get(h)).and_then(|p| p.as_player()).map(|p| p.invincibility_timer()).unwrap_or(0);
        if invincibility_timer <= 0 {
            self.collider.base.oc_flags1 |= OC1_TYPE_PLAYER;
        } else {
            return false;
        }
        if self.collider.base.oc_flags2 & OC2_HIT_PLAYER != 0 {
            self.collider.base.oc_flags2 &= !OC2_HIT_PLAYER;
            let arg2 = if self.action == Action::ContinueRolling { 1.5 } else { self.actor.speed_xz * 1.5 };
            // play->damagePlayer(play, -4).
            crate::player::play_damage_player(play, -4);
            actor_set_player_knockback_large_no_damage(play, arg2, self.actor.yaw_towards_player, 6.0);
            if let Some(p) = play.player {
                audio_play_actor_sfx_at(play, p, NA_SE_PL_BODY_HIT);
            }
            self.collider.base.oc_flags1 &= !OC1_TYPE_PLAYER;
        }
        false
    }

    /// `EnGo2_UpdateWaypoint`: on to the next waypoint (back from the reverse), from 0 to the
    /// path's count less two.
    fn update_waypoint(&mut self, play: &PlayState) -> i32 {
        let Some(path) = self.path_of(play) else { return 0 };
        let change = (path.count() as u8).wrapping_sub(1) as i32;
        if self.reverse != 0 {
            self.waypoint -= 1;
            if self.waypoint < 0 {
                self.waypoint = (change - 1) as i8;
            }
        } else {
            self.waypoint += 1;
            if self.waypoint as i32 >= change {
                self.waypoint = 0;
            }
        }
        1
    }

    /// `EnGo2_Orient`: turning to the waypoint; within 30 of it (and not on it), the next one.
    fn orient(&mut self, play: &PlayState) -> i32 {
        let (waypoint_dist_sq, target_yaw) = path_orient_and_get_dist_sq(&self.actor, self.path_of(play), self.waypoint as i16);
        // @bug (game): without a path targetYaw is never written (none of the rolling or walking
        // placements lacks one); taken as his yaw.
        let target_yaw = target_yaw.unwrap_or(self.actor.world_rot.y);
        smooth_step_to_s(&mut self.actor.world_rot.y, target_yaw, 6, 4000, 1);
        if waypoint_dist_sq > 0.0 && waypoint_dist_sq < 30.0 * 30.0 { self.update_waypoint(play) } else { 0 }
    }

    /// `EnGo2_IsWakingUp`: Link within 200 (Biggoron 800), under 60 (400) above or below and in
    /// front (Biggoron: touching him).
    fn is_waking_up(&mut self) -> bool {
        let ty = self.ty();
        let xyz_dist = if ty == GORON_DMT_BIGGORON { 800.0f32 } else { 200.0 };
        let y_dist = if ty == GORON_DMT_BIGGORON { 400.0 } else { 60.0 };
        if ty == GORON_DMT_BIGGORON {
            if self.collider.base.oc_flags2 & OC2_HIT_PLAYER == 0 {
                self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
                return false;
            } else {
                self.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED;
                return true;
            }
        }
        let xyz_dist = xyz_dist * xyz_dist;
        let yaw_diff = (self.actor.yaw_towards_player as f32 - self.actor.shape_rot.y as f32) as i32 as i16;
        let yaw_diff_abs = (yaw_diff as i32).abs() as i16;
        self.actor.xyz_dist_to_player_sq <= xyz_dist && self.actor.y_dist_to_player.abs() < y_dist && yaw_diff_abs < 0x2AA8
    }

    /// `EnGo2_IsRollingOnGround`: on the ground and not rising: a bounce (`arg1` of them, the
    /// first `arg2` high, `NA_SE_EN_GOLON_LAND_BIG` or `NA_SE_EN_DODO_M_GND`), then rolling on the
    /// ground for 60 to 89 frames (shaking 1.5 up and down with `arg3`); true while so.
    fn is_rolling_on_ground(&mut self, play: &mut PlayState, arg1: i16, arg2: f32, arg3: i16) -> bool {
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND == 0 || self.actor.velocity.y > 0.0 {
            return false;
        }
        if decr(&mut self.unk_590) != 0 {
            if arg3 == 0 {
                return true;
            }
            self.actor.world_pos.y = if self.unk_590 & 1 != 0 { self.actor.world_pos.y + 1.5 } else { self.actor.world_pos.y - 1.5 };
            audio_play_actor_sfx2(play, NA_SE_EV_BIGBALL_ROLL - SFX_FLAG);
            return true;
        }
        if self.unk_59c >= 2 {
            audio_play_actor_sfx2(play, if self.ty() == GORON_CITY_ROLLING_BIG { NA_SE_EN_GOLON_LAND_BIG } else { NA_SE_EN_DODO_M_GND });
        }
        self.unk_59c -= 1;
        if self.unk_59c <= 0 {
            if self.unk_59c == 0 {
                self.unk_590 = play.rand.s16_offset(60, 30);
                self.unk_59c = 0;
                self.actor.velocity.y = 0.0;
                return true;
            } else {
                self.unk_59c = arg1;
            }
        }
        self.actor.velocity.y = (self.unk_59c as f32 / arg1 as f32) * arg2;
        true
    }

    /// `EnGo2_BiggoronSetTextId` (Biggoron's; not ported).
    fn biggoron_set_text_id(&self) {
        if self.ty() == GORON_DMT_BIGGORON {
            log::debug!("En_Go2: EnGo2_BiggoronSetTextId (Biggoron, Phase 15) isn't ported");
        }
    }

    /// `func_80A45288`: the head and chest to Link (but walking out of the Fire Temple), and the
    /// talk once awake (but while the item's text shows).
    fn func_80a45288(&mut self, play: &mut PlayState) {
        if self.action != Action::GoronFireGenericAction {
            self.interact_info.target = play.player.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos).unwrap_or(self.actor.world_pos);
            // linkAge: LINK_AGE_ADULT 0, LINK_AGE_CHILD 1.
            let link_age = if play.save.adult { 0 } else { 1 };
            self.interact_info.eye_height = PLAYER_TRACKING_Y_OFFSETS[self.ty() as usize][link_age];
            let mut t = self.interact_info;
            track_point(play, &mut self.actor, &mut t, 4, self.tracking_mode as i16);
            self.interact_info = t;
        }
        if self.action != Action::SetGetItem && self.is_awake && self.func_80a44790(play) {
            self.biggoron_set_text_id();
        }
    }

    /// `func_80A45360`: the shadow fades in (0.4 of the way, at most 100 a frame) but while
    /// curled (`gGoronAnim_004930` up to frame 32).
    fn func_80a45360(&mut self) {
        let alpha_target = if self.skel.is(ANIM_004930) && self.skel.cur_frame <= 32.0 { 0.0 } else { 255.0 };
        approach_f(&mut self.alpha, alpha_target, 0.4, 100.0);
        self.actor.shape_shadow_alpha = self.alpha as u32 as u8;
    }

    /// `EnGo2_RollForward`: moving (but spinning in place), held still while talking.
    fn roll_forward(&mut self) {
        let speed_xz = self.actor.speed_xz;
        if self.interact_info.talk_state != NPC_TALK_STATE_IDLE {
            self.actor.speed_xz = 0.0;
        }
        if self.action != Action::ContinueRolling {
            // Actor_MoveXZGravity.
            self.actor.move_forward();
        }
        self.actor.speed_xz = speed_xz;
    }

    /// `func_80A454CC`: sitting up: standing (`gGoronAnim_0021D0`) for some, Biggoron's
    /// `gGoronAnim_003768` while he wants his sword mended; the others stay as they sat.
    fn func_80a454cc(&mut self, play: &PlayState) {
        match self.ty() {
            GORON_CITY_ROLLING_BIG | GORON_DMT_DC_ENTRANCE | GORON_CITY_ENTRANCE | GORON_CITY_STAIRWELL | GORON_DMT_FAIRY_HINT => self.change_anim(ENGO2_ANIM_9),
            GORON_DMT_BIGGORON
                if (ITEM_BROKEN_GORONS_SWORD..=ITEM_EYE_DROPS).contains(&play.save.inv_content(ITEM_TRADE_ADULT)) =>
            {
                self.change_anim(ENGO2_ANIM_4);
            }
            _ => self.skel.play_speed = 0.0,
        }
    }

    /// `EnGo2_GetTargetXZSpeed`: 3.6 for the big rolling Goron, 6 else (Link the Goron's 9 near
    /// Link).
    fn get_target_xz_speed(&self) -> f32 {
        let y_dist = if self.ty() == GORON_DMT_BIGGORON { 400.0 } else { 60.0 };
        let index = self.ty();
        if index == GORON_CITY_LINK && self.actor.y_dist_to_player.abs() < y_dist && self.actor.xz_dist_to_player < 400.0 {
            9.0
        } else if index == GORON_CITY_ROLLING_BIG {
            3.600_000_1
        } else {
            6.0
        }
    }

    /// `EnGo2_IsCameraModified`: whether he stays awake when Link leaves.
    fn is_camera_modified(&mut self, play: &PlayState) -> bool {
        let ty = self.ty();
        if ty == GORON_DMT_BIGGORON {
            log::debug!("En_Go2: Biggoron's camera (CAM_SET_DIRECTED_YAW, Phase 15) isn't ported");
        }
        if matches!(ty, GORON_FIRE_GENERIC | GORON_CITY_ROLLING_BIG | GORON_CITY_STAIRWELL | GORON_DMT_BIGGORON | GORON_MARKET_BAZAAR) {
            return true;
        }
        !play.save.check_quest_item(QUEST_MEDALLION_FIRE) && play.save.check_owned_equip(EQUIP_TYPE_TUNIC, EQUIP_INV_TUNIC_GORON)
    }

    /// `EnGo2_DefaultWakingUp`.
    fn default_waking_up(&mut self) {
        self.tracking_mode = if self.is_waking_up() { NPC_TRACKING_HEAD_AND_TORSO } else { NPC_TRACKING_NONE };
        if self.interact_info.talk_state != NPC_TALK_STATE_IDLE {
            self.tracking_mode = NPC_TRACKING_FULL_BODY;
        }
        self.is_awake = true;
    }

    /// `EnGo2_WakingUp`.
    fn waking_up(&mut self) {
        let xyz_dist = if self.ty() == GORON_DMT_BIGGORON { 800.0f32 } else { 200.0 };
        let xyz_dist = xyz_dist * xyz_dist;
        self.tracking_mode = NPC_TRACKING_NONE;
        if self.actor.xyz_dist_to_player_sq <= xyz_dist || self.interact_info.talk_state != NPC_TALK_STATE_IDLE {
            self.tracking_mode = NPC_TRACKING_FULL_BODY;
        }
        self.is_awake = true;
    }

    /// `EnGo2_BiggoronWakingUp`.
    fn biggoron_waking_up(&mut self) {
        if self.is_waking_up() || self.interact_info.talk_state != NPC_TALK_STATE_IDLE {
            self.tracking_mode = NPC_TRACKING_HEAD_AND_TORSO;
            self.is_awake = true;
        } else {
            self.tracking_mode = NPC_TRACKING_NONE;
            self.is_awake = false;
        }
    }

    /// `EnGo2_SelectGoronWakingUp`.
    fn select_goron_waking_up(&mut self, play: &PlayState) {
        match self.ty() {
            GORON_DMT_BOMB_FLOWER => {
                self.is_awake = true;
                self.tracking_mode = if self.is_waking_up() { NPC_TRACKING_HEAD_AND_TORSO } else { NPC_TRACKING_NONE };
            }
            GORON_FIRE_GENERIC => self.waking_up(),
            GORON_DMT_BIGGORON => self.biggoron_waking_up(),
            GORON_CITY_LINK if !play.save.check_quest_item(QUEST_MEDALLION_FIRE) && play.save.check_owned_equip(EQUIP_TYPE_TUNIC, EQUIP_INV_TUNIC_GORON) => self.waking_up(),
            _ => self.default_waking_up(),
        }
    }

    /// `EnGo2_EyeMouthTexState`: fixed faces, or blinking.
    fn eye_mouth_tex_state(&mut self, play: &mut PlayState) {
        match self.eye_mouth_tex_state {
            1 => {
                self.blink_timer = 0;
                self.eye_tex_index = 0;
                self.mouth_tex_index = 0;
            }
            2 => {
                self.blink_timer = 0;
                self.eye_tex_index = 1;
                self.mouth_tex_index = 0;
            }
            // Only when Biggoron is given the eye drops: he smiles (the second mouth's only use).
            3 => {
                self.blink_timer = 0;
                self.eye_tex_index = 0;
                self.mouth_tex_index = 1;
            }
            _ => {
                if decr(&mut self.blink_timer) == 0 {
                    self.eye_tex_index += 1;
                    if self.eye_tex_index >= 4 {
                        self.blink_timer = play.rand.s16_offset(30, 30);
                        self.eye_tex_index = 1;
                    }
                }
            }
        }
    }

    /// `EnGo2_SitDownAnimation`: `gGoronAnim_004930`'s sounds (uncurling at frame 14; curling at 40
    /// and 1).
    fn sit_down_animation(&mut self, play: &mut PlayState) {
        if self.skel.play_speed != 0.0 && self.skel.is(ANIM_004930) {
            if self.skel.play_speed > 0.0 && self.skel.cur_frame == 14.0 {
                if self.ty() != GORON_DMT_BIGGORON {
                    audio_play_actor_sfx2(play, NA_SE_EN_GOLON_SIT_DOWN);
                } else {
                    play.audio.func_800f4524(SfxPos::Default, NA_SE_EN_GOLON_SIT_DOWN, 60);
                }
            }
            if self.skel.play_speed < 0.0 {
                if self.skel.cur_frame == 1.0 {
                    audio_play_actor_sfx2(play, NA_SE_EN_DODO_M_GND);
                }
                if self.skel.cur_frame == 40.0 {
                    audio_play_actor_sfx2(play, NA_SE_EN_GOLON_SIT_DOWN);
                }
            }
        }
    }

    /// `EnGo2_RollingAnimation`: curling up (`gGoronAnim_004930` backwards from its end).
    fn rolling_animation(&mut self) {
        if self.ty() == GORON_DMT_BIGGORON {
            self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
            self.change_anim(ENGO2_ANIM_10);
            self.skel.play_speed = -0.5;
        } else {
            self.change_anim(ENGO2_ANIM_1);
            self.skel.play_speed = -1.0;
        }
        self.swap_initial_frame_anim_frame_count();
        self.tracking_mode = NPC_TRACKING_NONE;
        self.unk_211 = false;
        self.is_awake = false;
        self.action = Action::CurledUp;
    }

    /// `EnGo2_WakeUp`: uncurling (`NA_SE_EN_GOLON_WAKE_UP` from still).
    fn wake_up(&mut self, play: &mut PlayState) {
        if self.skel.play_speed == 0.0 {
            if self.ty() != GORON_DMT_BIGGORON {
                audio_play_actor_sfx2(play, NA_SE_EN_GOLON_WAKE_UP);
            } else {
                play.audio.func_800f4524(SfxPos::Default, NA_SE_EN_GOLON_WAKE_UP, 60);
            }
        }
        if self.ty() == GORON_DMT_BIGGORON {
            log::debug!("En_Go2: Biggoron's one-point cutscene 4200 (Phase 15) isn't ported");
            self.change_anim(ENGO2_ANIM_10);
            self.skel.play_speed = 0.5;
        } else {
            self.change_anim(ENGO2_ANIM_1);
            self.skel.play_speed = 1.0;
        }
        self.action = Action::Func80A46B40;
    }

    /// `EnGo2_GetItemAnimation`: awake and sitting at once (`gGoronAnim_004930`'s end).
    fn get_item_animation(&mut self) {
        self.change_anim(ENGO2_ANIM_1);
        self.unk_211 = true;
        self.action = Action::Func80A46B40;
        self.skel.play_speed = 0.0;
        self.actor.speed_xz = 0.0;
        self.skel.cur_frame = self.skel.end_frame;
    }

    /// `EnGo2_SetupRolling`: spinning up (twice his speed for ten frames), 1800 up; the big one's
    /// and Link the Goron's cylinder takes hits.
    fn setup_rolling(&mut self, play: &PlayState) {
        let ty = self.ty();
        if ty == GORON_CITY_ROLLING_BIG || ty == GORON_CITY_LINK {
            self.collider.info.ac_elem_flags = ACELEM_ON;
            self.actor.speed_xz = if play.save.get_inf_table(INFTABLE_11E) { 6.0 } else { 3.600_000_1 };
        } else {
            self.actor.speed_xz = 6.0;
        }
        self.actor.flags |= ACTOR_FLAG_SFX_FOR_PLAYER_BODY_HIT;
        self.anim_timer = 10;
        self.actor.shape_y_offset = 1800.0;
        // Speeding up.
        self.actor.speed_xz *= 2.0;
        self.action = Action::ContinueRolling;
    }

    /// `EnGo2_StopRolling`: the trail's small one drops a bomb that blows at once; the big one's
    /// cylinder stops taking hits.
    fn stop_rolling(&mut self, play: &mut PlayState) {
        let ty = self.ty();
        if ty != GORON_CITY_ROLLING_BIG && ty != GORON_CITY_LINK {
            if ty == GORON_DMT_ROLLING_SMALL {
                match play.actor_spawn(ACTOR_EN_BOM, self.actor.world_pos, [0, 0, 0], 0) {
                    Ok(h) => {
                        if let Some(bomb) = play.actors.downcast_mut::<EnBom>(h) {
                            bomb.timer = 0;
                        }
                    }
                    Err(e) => log::debug!("En_Go2's bomb: {e:?}"),
                }
            }
        } else {
            self.collider.info.ac_elem_flags = ACELEM_NONE;
        }
        self.actor.shape_rot = self.actor.world_rot;
        self.unk_59c = 0;
        self.unk_590 = 0;
        self.action = Action::GroundRolling;
        self.actor.shape_y_offset = 0.0;
        self.actor.speed_xz = 0.0;
    }

    /// `EnGo2_IsFreeingGoronInFire` (the Fire Temple's; not ported).
    fn is_freeing_goron_in_fire(&self) -> bool {
        if self.ty() != GORON_FIRE_GENERIC {
            return false;
        }
        log::debug!("En_Go2: EnGo2_IsFreeingGoronInFire (the Fire Temple's Gorons, Phase 15) isn't ported");
        false
    }

    /// `EnGo2_IsGoronDmtBombFlower`: the bomb-flower Goron's walk after 0x300B.
    fn is_goron_dmt_bomb_flower(&mut self) -> bool {
        if self.ty() != GORON_DMT_BOMB_FLOWER || self.interact_info.talk_state != NPC_TALK_STATE_ACTION {
            return false;
        }
        self.change_anim(ENGO2_ANIM_3);
        self.interact_info.talk_state = NPC_TALK_STATE_IDLE;
        self.is_awake = false;
        self.tracking_mode = NPC_TRACKING_NONE;
        self.action = Action::GoronDmtBombFlowerAnimation;
        true
    }

    /// `EnGo2_IsGoronRollingBig`: the big rolling Goron rolls on after his talk.
    fn is_goron_rolling_big(&mut self) -> bool {
        if self.ty() != GORON_CITY_ROLLING_BIG || self.interact_info.talk_state != NPC_TALK_STATE_ACTION {
            return false;
        }
        self.interact_info.talk_state = NPC_TALK_STATE_IDLE;
        self.rolling_animation();
        self.action = Action::GoronRollingBigContinueRolling;
        true
    }

    /// `EnGo2_IsGoronFireGeneric` (the Fire Temple's; not ported).
    fn is_goron_fire_generic(&self) -> bool {
        if self.ty() != GORON_FIRE_GENERIC || self.interact_info.talk_state == NPC_TALK_STATE_IDLE {
            return false;
        }
        log::debug!("En_Go2: EnGo2_IsGoronFireGeneric (the Fire Temple's Gorons, Phase 15) isn't ported");
        false
    }

    /// `EnGo2_IsGoronLinkReversing` (Link the Goron's; not ported).
    fn is_goron_link_reversing(&mut self) -> bool {
        if self.ty() != GORON_CITY_LINK || self.waypoint as i32 >= self.unk_216 as i32 || !self.is_waking_up() {
            return false;
        }
        log::debug!("En_Go2: EnGo2_IsGoronLinkReversing (Link the Goron, Phase 15) isn't ported");
        false
    }

    /// `EnGo2_IsRolling`: rolling while talking (Link the Goron's).
    fn is_rolling(&mut self, play: &mut PlayState) -> bool {
        if self.interact_info.talk_state == NPC_TALK_STATE_IDLE || self.actor.speed_xz < 1.0 {
            return false;
        }
        if self.is_rolling_on_ground(play, 2, (20.0f64 / 3.0) as f32, 0) {
            if self.unk_590 >= 9 && self.unk_59c == 0 {
                self.unk_590 = 8;
            }
            self.get_dust_data(play, 0);
        }
        true
    }

    /// `EnGo2_GoronLinkAnimation` (Link the Goron's; not ported).
    fn goron_link_animation(&self) {
        if self.ty() == GORON_CITY_LINK {
            log::debug!("En_Go2: EnGo2_GoronLinkAnimation (Link the Goron, Phase 15) isn't ported");
        }
    }

    /// `EnGo2_BiggoronAnimation` (Biggoron's; not ported).
    fn biggoron_animation(&self, play: &PlayState) {
        if (ITEM_BROKEN_GORONS_SWORD..=ITEM_EYE_DROPS).contains(&play.save.inv_content(ITEM_TRADE_ADULT))
            && self.ty() == GORON_DMT_BIGGORON
            && self.interact_info.talk_state == NPC_TALK_STATE_IDLE
        {
            log::debug!("En_Go2: EnGo2_BiggoronAnimation (Biggoron, Phase 15) isn't ported");
        }
    }

    /// `EnGo2_CurledUp`: curling (the dust as it ends), the cylinder shrinking with it; waking up.
    fn curled_up(&mut self, play: &mut PlayState) {
        let index = self.ty() as usize;
        if self.on_end_frame() {
            if self.ty() == GORON_DMT_BIGGORON {
                log::debug!("En_Go2: Biggoron's quake (QUAKE_TYPE_3, Phase 15) isn't ported");
            } else {
                self.get_dust_data(play, 1);
            }
            self.skel.play_speed = 0.0;
        }
        if self.skel.cur_frame as i32 == 0 {
            self.collider.dim.height = (D_80A4816C[index].height as f32 * 0.6) as i16;
        } else {
            let height = D_80A4816C[index].height;
            self.collider.dim.height = ((D_80A4816C[index].height as f32 * 0.4 * (self.skel.cur_frame / self.skel.start_frame)) + (height as f32 * 0.6)) as i16;
        }
        if self.is_freeing_goron_in_fire() {
            self.is_awake = false;
            self.wake_up(play);
        }
        if self.ty() != GORON_FIRE_GENERIC && self.is_waking_up() {
            self.wake_up(play);
        }
    }

    /// `func_80A46B40`: uncurling (the cylinder growing), then awake: tracking, the talk's actions,
    /// curling up again when Link leaves.
    fn func_80a46b40(&mut self, play: &mut PlayState) {
        let index = self.ty() as usize;
        if self.unk_211 {
            self.biggoron_animation(play);
            self.goron_link_animation();
            self.select_goron_waking_up(play);
            if !self.is_goron_rolling_big() && !self.is_goron_fire_generic() {
                if self.is_goron_dmt_bomb_flower() {
                    return;
                }
            } else {
                return;
            }
        } else if self.on_end_frame() {
            if self.ty() == GORON_DMT_BIGGORON {
                self.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED;
            }
            self.func_80a454cc(play);
            self.unk_211 = true;
            self.collider.dim.height = D_80A4816C[index].height;
        } else {
            let height = D_80A4816C[index].height as f32;
            self.collider.dim.height = ((height * 0.4 * (self.skel.cur_frame / self.skel.end_frame)) + (height * 0.6)) as i16;
        }
        if !self.is_camera_modified(play) && !self.is_waking_up() {
            self.rolling_animation();
        }
    }

    /// `EnGo2_GoronDmtBombFlowerAnimation`: walking (a step a cycle of `gGoronAnim_010590`) to the
    /// waypoint; back at waypoint 0 he sits.
    fn goron_dmt_bomb_flower_animation(&mut self, play: &mut PlayState) {
        let float1 = self.skel.end_frame;
        let float2 = self.skel.cur_frame * (32768.0 / float1);
        self.actor.speed_xz = sin_s(float2 as i32 as i16);
        if self.orient(play) != 0 && self.waypoint == 0 {
            self.get_item_animation();
        }
    }

    /// `EnGo2_GoronRollingBigContinueRolling`: curled, he rolls on.
    fn goron_rolling_big_continue_rolling(&mut self, play: &mut PlayState) {
        if self.on_end_frame() {
            self.get_dust_data(play, 1);
            self.skel.play_speed = 0.0;
            self.setup_rolling(play);
        }
    }

    /// `EnGo2_ContinueRolling`: spinning in place ten frames (the trail's small one only once
    /// Link is within 1000), then half the speed and away.
    fn continue_rolling(&mut self, play: &mut PlayState) {
        let float1 = 1000.0f32;
        if (self.ty() != GORON_DMT_ROLLING_SMALL || !(self.actor.xyz_dist_to_player_sq > float1 * float1)) && decr(&mut self.anim_timer) == 0 {
            self.action = Action::SlowRolling;
            // Slowdown.
            self.actor.speed_xz *= 0.5;
        }
        self.get_dust_data(play, 2);
    }

    /// `EnGo2_SlowRolling`: along the path, bouncing; the trail's small one stops back at
    /// waypoint 0.
    fn slow_rolling(&mut self, play: &mut PlayState) {
        if self.is_rolling(play) {
            return;
        }
        if self.is_rolling_on_ground(play, 4, 8.0, 1) {
            if self.is_goron_link_reversing() {
                self.action = Action::ReverseRolling;
                return;
            }
            self.get_dust_data(play, 3);
        }
        let orientation = self.orient(play);
        let index = self.ty();
        if index != GORON_CITY_LINK {
            if index == GORON_DMT_ROLLING_SMALL && orientation == 1 && self.waypoint == 0 {
                self.stop_rolling(play);
                return;
            }
        } else if orientation == 2 && self.waypoint == 1 {
            // @bug (game): EnGo2_Orient returns 0 or 1: never so.
            self.stop_rolling(play);
            return;
        }
        let target = self.get_target_xz_speed();
        approach_f(&mut self.actor.speed_xz, target, 0.4, 0.6);
        self.actor.shape_rot = self.actor.world_rot;
    }

    /// `EnGo2_GroundRolling`: settling (the bounces), then curled (the big one wakes).
    fn ground_rolling(&mut self, play: &mut PlayState) {
        if self.is_rolling_on_ground(play, 4, 8.0, 0) {
            self.get_dust_data(play, 0);
            if self.unk_59c == 0 {
                match self.ty() {
                    GORON_CITY_LINK => {
                        self.goron_state = 0;
                        self.action = Action::GoronLinkStopRolling;
                    }
                    GORON_CITY_ROLLING_BIG => self.wake_up(play),
                    _ => self.action = Action::CurledUp,
                }
            }
        }
    }

    /// `EnGo2_SetupGetItem`: offering the item until Link takes it.
    fn setup_get_item(&mut self, play: &mut PlayState) {
        if actor_has_parent(&self.actor) {
            self.actor.parent = None;
            self.action = Action::SetGetItem;
        } else {
            offer_get_item_range(play, &self.actor, self.get_item_id as i16, self.actor.xz_dist_to_player + 1.0, self.actor.y_dist_to_player.abs() + 1.0);
        }
    }

    /// `EnGo2_SetGetItem`: the item's text read; the big rolling Goron rolls on.
    fn set_get_item(&mut self, play: &mut PlayState) {
        if play.message_state() == TEXT_STATE_DONE && play.message_should_advance() {
            self.interact_info.talk_state = NPC_TALK_STATE_IDLE;
            match self.get_item_id as i16 {
                GI_CLAIM_CHECK => {
                    log::debug!("En_Go2: the claim check's Environment_ClearBgsDayCount (Biggoron, Phase 15) isn't ported");
                    self.get_item_animation();
                    return;
                }
                GI_TUNIC_GORON => {
                    log::debug!("En_Go2: the Goron Tunic's INFTABLE_109 (Link the Goron, Phase 15) isn't ported");
                    self.get_item_animation();
                    return;
                }
                GI_SWORD_BIGGORON => {
                    log::debug!("En_Go2: Biggoron's Sword's bgsFlag (Biggoron, Phase 15) isn't ported");
                }
                GI_BOMB_BAG_30 | GI_BOMB_BAG_40 => {
                    self.rolling_animation();
                    self.action = Action::GoronRollingBigContinueRolling;
                    return;
                }
                _ => {}
            }
            self.action = Action::Func80A46B40;
        }
    }

    /// The limb matrices `EnGo2_Draw` draws the skeleton with: `EnGo2_OverrideLimbDraw`'s head and
    /// chest turns and the sway of limbs 10, 11 and 14 (`sway`: limb, y angle, z angle).
    fn pose(skeleton: &Skeleton, joints: &[[i16; 3]], head: [i16; 2], torso: [i16; 2], sway: &[(usize, i16, i16)]) -> Vec<Mat4> {
        // BINANG_TO_RAD_ALT.
        let r = binang_to_rad;
        skeleton.pose_override(joints, |limb, _pos, rot| {
            let mut pre = Mat4::IDENTITY;
            if limb == LIMB_HEAD {
                // Translate(2800, 0, 0), RotateX(headRot.y), RotateZ(headRot.x), Translate(-2800, 0, 0).
                pre = Mat4::from_translation(Vec3::new(2800.0, 0.0, 0.0)) * Mat4::from_rotation_x(r(head[1])) * Mat4::from_rotation_z(r(head[0])) * Mat4::from_translation(Vec3::new(-2800.0, 0.0, 0.0));
            }
            if limb == LIMB_CHEST {
                // RotateY(torsoRot.y), RotateX(torsoRot.x).
                pre = Mat4::from_rotation_y(r(torso[1])) * Mat4::from_rotation_x(r(torso[0]));
            }
            if let Some(&(_, a, b)) = sway.iter().find(|s| s.0 == limb) {
                rot[1] = (rot[1] as f32 + sin_s(a) * FIDGET_AMPLITUDE) as i32 as i16;
                rot[2] = (rot[2] as f32 + cos_s(b) * FIDGET_AMPLITUDE) as i32 as i16;
            }
            pre
        })
    }

    fn sway_of(y: &[i16; GORON_LIMB_MAX], z: &[i16; GORON_LIMB_MAX]) -> [(usize, i16, i16); 3] {
        SWAY_LIMBS.map(|l| (l, y[l], z[l]))
    }

    /// Which of `EnGo2_Draw`'s draws: curled (`EnGo2_DrawCurledUp`), rolling (`EnGo2_DrawRolling`)
    /// or the skeleton.
    pub fn draw_mode(&self) -> u32 {
        if self.action == Action::CurledUp && self.skel.play_speed == 0.0 && self.skel.cur_frame == 0.0 {
            rs::CURLED
        } else if matches!(self.action, Action::SlowRolling | Action::ReverseRolling | Action::ContinueRolling) {
            rs::ROLLING
        } else {
            rs::SKELETON
        }
    }

    /// `EnGo2_DrawEffects`: each puff facing the camera, its alpha and dust frame by the time it
    /// has left.
    fn draw_effects(&self, view: &ViewInfo, out: &mut DrawOut) {
        for e in self.effects.iter().filter(|e| e.type_ != 0) {
            let alpha = (e.timer as f32 * (255.0 / e.initial_timer as f32)) as i32 as i16;
            let index = (e.timer as f32 * (8.0 / e.initial_timer as f32)) as i32 as i16;
            let m = oot_game::draw::replace_rotation(Mat4::from_translation(e.pos), view.billboard) * Mat4::from_scale(Vec3::new(e.scale, e.scale, 1.0));
            let mut sv = SegmentValues::default();
            sv.prim[SEG_DUST_COLOR as usize] = Some([DUST_PRIM[0], DUST_PRIM[1], DUST_PRIM[2], alpha as u8]);
            sv.env[SEG_DUST_COLOR as usize] = Some(DUST_ENV);
            let index = (index.max(0) as usize).min(DUST_TEXTURES.len() - 1);
            out.xlu.push(DrawCmd { mesh: MeshKey::named(keys::bake(&dust_bake_name(index))), transform: m, bones: Vec::new(), params: DrawParams { segments: Some(sv), ..Default::default() } });
        }
    }
}

/// Indices into the render state's extras.
mod rs {
    /// `angles`: head x, y; torso x, y; the sway (limbs 10, 11, 14: y, z each); the roll.
    pub const HEAD: usize = 0;
    pub const TORSO: usize = 2;
    pub const SWAY: usize = 4;
    pub const ROLL: usize = 10;
    pub const ANGLES: usize = 11;
    /// `switches`: the draw, the eye, the mouth.
    pub const MODE: usize = 0;
    pub const EYE: usize = 1;
    pub const MOUTH: usize = 2;
    pub const SWITCHES: usize = 3;
    /// The draws.
    pub const SKELETON: u32 = 0;
    pub const CURLED: u32 = 1;
    pub const ROLLING: u32 = 2;
}

impl ActorImpl for EnGo2 {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `EnGo2_Update`.
    fn update(&mut self, play: &mut PlayState) {
        self.func_80a45360();
        self.sit_down_animation(play);
        self.skel.update();
        self.roll_forward();
        let (h, r) = (self.collider.dim.height as f32 * 0.5, self.collider.dim.radius as f32 * 0.6);
        self.actor.update_bg_check_info(&play.col, h, r, 0.0, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2);
        // OOT_VERSION >= PAL_1_0: only while not talking.
        if self.interact_info.talk_state == NPC_TALK_STATE_IDLE {
            self.func_80a44ab0(play);
        }
        match self.action {
            Action::CurledUp => self.curled_up(play),
            Action::Func80A46B40 => self.func_80a46b40(play),
            Action::GoronDmtBombFlowerAnimation => self.goron_dmt_bomb_flower_animation(play),
            Action::GoronRollingBigContinueRolling => self.goron_rolling_big_continue_rolling(play),
            Action::ContinueRolling => self.continue_rolling(play),
            Action::SlowRolling => self.slow_rolling(play),
            Action::GroundRolling => self.ground_rolling(play),
            Action::SetupGetItem => self.setup_get_item(play),
            Action::SetGetItem => self.set_get_item(play),
            a @ (Action::ReverseRolling | Action::BiggoronEyedrops | Action::GoronLinkStopRolling | Action::GoronFireGenericAction) => {
                log::debug!("En_Go2: EnGo2_{a:?} (the adult's Gorons, Phase 15) isn't ported");
            }
        }
        if self.unk_211 {
            actor_update_fidget_tables(play, &mut self.fidget_table_y, &mut self.fidget_table_z, GORON_LIMB_MAX);
        }
        self.func_80a45288(play);
        self.eye_mouth_tex_state(play);
        self.check_collision(play);
    }
    /// What `EnGo2_Draw` changes: the dust (`EnGo2_UpdateEffects`), the focus (the curled and
    /// rolling lists' origin; `EnGo2_PostLimbDraw`'s 600 along the head), the roll's spin.
    fn draw_update(&mut self, play: &mut PlayState) {
        self.update_effects(play);
        let speed_xz = if self.action == Action::ReverseRolling { 0.0 } else { self.actor.speed_xz };
        self.roll_angle = play.state_frames.wrapping_mul(((speed_xz as i32 as i16 as i32) * 1400) as u32) as i16;
        let m = actor_draw_matrix(&RenderState::of(&self.actor));
        if self.draw_mode() == rs::SKELETON {
            let t = &self.interact_info;
            let bones = Self::pose(&self.skeleton, &self.skel.joint_table, [t.head[0], t.head[1]], [t.torso[0], t.torso[1]], &Self::sway_of(&self.fidget_table_y, &self.fidget_table_z));
            if let Some(head) = bones.get(LIMB_HEAD - 1) {
                self.actor.focus_pos = (m * *head).transform_point3(Vec3::new(600.0, 0.0, 0.0));
            }
        } else {
            // Matrix_MultVec3f(&{ 0, 0, 0 }, &focus.pos).
            self.actor.focus_pos = m.transform_point3(Vec3::ZERO);
        }
    }
    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        let t = &self.interact_info;
        let sway = Self::sway_of(&self.fidget_table_y, &self.fidget_table_z);
        rs.angles = vec![t.head[0], t.head[1], t.torso[0], t.torso[1], sway[0].1, sway[0].2, sway[1].1, sway[1].2, sway[2].1, sway[2].2, self.roll_angle];
        rs.switches = vec![self.draw_mode(), (self.eye_tex_index as u32).min(3), (self.mouth_tex_index as u32).min(1)];
        rs
    }
    /// `EnGo2_Draw`: the dust, then curled, rolling or the skeleton (`Gfx_SetupDL_25Opa`, the eyes
    /// on 8, the mouth on 9).
    fn draw(&self, rs: &RenderState, _play: &PlayState, view: &ViewInfo, out: &mut DrawOut) {
        self.draw_effects(view, out);
        let Some(joints) = &rs.joints else { return };
        if rs.angles.len() < rs::ANGLES || rs.switches.len() < rs::SWITCHES {
            return;
        }
        let a = &rs.angles;
        let m = actor_draw_matrix(rs);
        match rs.switches[rs::MODE] {
            rs::CURLED => out.opa.push(DrawCmd::new(MeshKey::named(keys::mesh(OBJECT, CURLED_DL)), m)),
            rs::ROLLING => {
                // Matrix_RotateZYX(frames * ((s16)speedXZ * 1400), 0, shape.rot.z, MTXMODE_APPLY).
                let spin = Mat4::from_rotation_z(binang_to_rad(rs.rot[2])) * Mat4::from_rotation_x(binang_to_rad(a[rs::ROLL]));
                out.opa.push(DrawCmd::new(MeshKey::named(keys::mesh(OBJECT, ROLLING_DL)), m * spin));
            }
            _ => {
                let sway = [(SWAY_LIMBS[0], a[rs::SWAY], a[rs::SWAY + 1]), (SWAY_LIMBS[1], a[rs::SWAY + 2], a[rs::SWAY + 3]), (SWAY_LIMBS[2], a[rs::SWAY + 4], a[rs::SWAY + 5])];
                let bones = Self::pose(&self.skeleton, &joints.rot, [a[rs::HEAD], a[rs::HEAD + 1]], [a[rs::TORSO], a[rs::TORSO + 1]], &sway);
                let key = MeshKey::named(keys::bake(&bake_name(rs.switches[rs::EYE] as usize, rs.switches[rs::MOUTH] as usize)));
                out.opa.push(DrawCmd { mesh: key, transform: m, bones, params: Default::default() });
            }
        }
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
    fn the_bakes_are_every_face_and_dust_frame() {
        let b = bakes();
        assert_eq!(b.len(), 4 * 2 + 8);
        assert!(b.iter().any(|m| m.name == bake_name(3, 1)));
        assert!(b.iter().any(|m| m.name == dust_bake_name(7)));
    }

    #[test]
    fn the_tables_cover_every_type() {
        assert_eq!(D_80A4816C.len(), (GORON_MARKET_BAZAAR + 1) as usize);
        assert_eq!(D_80A481F8.len(), (GORON_MARKET_BAZAAR + 1) as usize);
        assert_eq!(PLAYER_TRACKING_Y_OFFSETS.len(), (GORON_MARKET_BAZAAR + 1) as usize);
        // The bomb-flower Goron's collider and talk range: 46 + 30.
        assert_eq!(D_80A4816C[4].radius as f32 + D_80A481F8[4].interact_range, 76.0);
    }
}
