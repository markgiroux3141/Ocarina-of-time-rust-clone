//! `En_In` (`ovl_En_In/z_en_in.c`): Ingo, Lon Lon Ranch's hand.
//!
//! **Where he is** (`EnIn_GetStartMode`, by the scene, Link's age, the time, `shape.rot.z` and the
//! story): for a child he works (`ENIN_START_MODE_WORKING`: `gIngoWorkingAnim`, the basket and the
//! pitchfork in his hands) in the ranch by day until Talon is back from the castle (rot.z 1),
//! then in the stable by day (rot.z 3), and in the stable by night before (2) and after (4). For an
//! adult he runs the ranch by day until Link has Epona (`ENIN_START_MODE_RUNNING_RANCH`, rot.z 5:
//! the horse rental and the race), is obedient there after (`_OBEDIENT`, rot.z 7), and sleeps in
//! Lon Lon's buildings by night (`_SLEEPING`, rot.z 6 before Epona, 8 after). Anything else
//! (`ENIN_START_MODE_NULL`) is killed on the first update. The race's Ingo on horseback is one
//! `En_Horse` spawns (params 1, rot.z 1: `EnIn_StartingRace`).
//!
//! `EnIn_Init` resets `eventInf[EVENTINF_INDEX_HORSES]` (the race's state and flags, and the
//! cucco game's) the first time an Ingo spawns while Link's void-out respawn point
//! (`respawn[RESPAWN_MODE_DOWN]`) is the ranch's entrance (1107, 0, -3740) (`sIsObedient`, the
//! overlay's static: ADR 0059's `overlay_static`). `EnIn_WaitForObject`, his first update, sets him
//! up (it waits for `OBJECT_IN`, his profile's object, so never more than that frame).
//!
//! He talks (`Npc_UpdateTalking` within 80 or 320 plus his radius, by his attention range): his
//! text by age (`EnIn_GetTextId`: a child's 0x203F then 0x2040 before Talon's return, 0x2045 then
//! 0x2046 after; an adult's by Epona, the time and the race's state, `EnIn_GetTextIdAdult`), and
//! the flags and choices of each text as the box goes (`EnIn_UpdateTalkState`: the rental's price,
//! the rematch's). He turns his head and chest to Link (`EnIn_UpdateTracking`, preset 1), blinks
//! (`EnIn_UpdateEyes`; eye 3, closed for good, when obedient), and sways his chest and shoulders
//! (`EnIn_Fidget`; still while working: then their fixed `Math_CosS(0)` tilt of 200).
//!
//! Drawn as `EnIn_Draw` does: his eyes on segment 8, `gIngoHeadGradient2Tex` on 9, his head and
//! chest turned (`EnIn_OverrideLimbDraw`), the child-era basket and pitchfork on his hands while
//! working (`EnIn_PostLimbDraw`), the adult-era body for `params > 0` (`sAdultEraDLs`, every limb
//! but the head), from meshes baked per eye and era (docs/adr/0012-actor-bakes.md).
//!
//! **Ported:** the child's Ingo whole (`EnIn_Init`, `EnIn_WaitForObject` with `EnIn_StartingRace`
//! and `EnIn_GetStartMode` for both ages, `EnIn_Idle`, the texts and `EnIn_UpdateTalkState` with its
//! helpers for every text, `EnIn_UpdateTracking`, `EnIn_UpdateEyes`, `EnIn_Fidget`, `EnIn_Stagger`,
//! `EnIn_Update`, the draw).
//! **Not ported** (the adult's race and rental, Phase 13): `EnIn_Racing`, `EnIn_StartRentalPeriod`,
//! `EnIn_RentalPeriod`, `EnIn_OfferingRematch`, `EnIn_PlayerLostRace`, `EnIn_PlayerWonFirstRace`,
//! `EnIn_PlayerWonSecondRace`, `EnIn_ClosingGate`, `EnIn_FinishClosingGate`, `EnIn_GuardingGate`,
//! `EnIn_PlayerEscapesRanch`, `EnIn_EndRentalPeriod`, `EnIn_StartTransition` and
//! `EnIn_RentalTimeExpired`. Where `EnIn_WaitForObject` would start the race (`EnIn_StartingRace`)
//! or an adult's start mode (`_RUNNING_RANCH` that stays, `_OBEDIENT`, `_SLEEPING`), his init
//! spawns a placeholder instead (the port can't turn a live actor into one on its first update, so
//! `EnIn_Init` makes the same checks), so the adult's texts (`EnIn_GetTextIdAdult`, his choices'
//! replies) are here but never show. The mask reactions (`MaskReaction_GetTextId`, 0x717C to
//! 0x717F: taken as 0, so none).
//!
//! Params: `params` whole: -1 placed (the scene, the age, the time and rot.z decide); 1 the race's
//! rider (`En_Horse` spawns him); 2 the ranch's adult Ingo offering the rental or the race (race
//! states 0, 2, 3, 4, 7), 3 in the rental period (state 1), 4 trapped by his loss (states 5, 6)
//! (placed in the ranch's adult day layer, rot.z 5); set to 5 by his init asleep before Epona. Over
//! 0 he's drawn with the adult-era body and is killed if `OBJECT_IN` isn't in a bank.
//! `shape.rot.z`: 1 to 8, which Ingo (above).
//! Flags: reads EVENTCHKINF_TALON_RETURNED_FROM_CASTLE (where he is, the child's texts) and
//! EVENTCHKINF_EPONA_OBTAINED (where he is, the adult's texts, asleep's params 5); reads and sets
//! INFTABLE_INGO_TALKED_TO_CHILD_LINK_BEFORE_TALON_RETURNED (0x203F closed, which also sets
//! EVENTCHKINF_11) and INFTABLE_INGO_TALKED_TO_CHILD_LINK_AFTER_TALON_RETURNED (0x2045 closed);
//! reads and sets INFTABLE_INGO_TALKED_TO_ADULT_LINK (0x2030 or 0x2031's choice); reads and sets
//! INFTABLE_A2 (0x2038's choice; the rental clears it: not ported) and reads EVENTCHKINF_1B (the
//! rental's texts; the rental sets it: not ported); the rental (not ported) reads and sets
//! INFTABLE_AB. `eventInf[EVENTINF_INDEX_HORSES]`: the race's state (bits 0 to 3, read by the start
//! and the adult's texts, 0x205B's "no" sets INGO_RACE_STATE_OFFER_RENTAL), EVENTINF_INGO_RACE_LOST_ONCE
//! and EVENTINF_INGO_RACE_SECOND_RACE (read by the adult's texts, cleared by 0x205B's "no", set by
//! the race: not ported), EVENTINF_INGO_RACE_HORSETYPE and EVENTINF_INGO_RACE_0F (written by the
//! race: not ported); the whole word cleared by `sIsObedient`'s reset and the rental's start.

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, MeshKey};
use eng_math::{binang_to_rad, cos_s, sin_s};
use glam::{Mat4, Vec3};
use oot_game::actor::{ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_FRIENDLY, ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor, UPDBGCHECKINFO_FLAG_2};
use oot_game::actor_ctx::{ACTORCAT_NPC, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind, Source, TextLine};
use oot_game::{event_chk_inf, event_inf, inf_table};
use oot_game::collision_check::*;
use oot_game::message::*;
use oot_game::npc::{NpcTrack, process_talk_request, talk_update, track_point};
use oot_game::pack::{BakeBody, BakeSegment, LimbOverride, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::save::{RESPAWN_MODE_DOWN, SUBTIMER_STATE_OFF};
use oot_game::skelanime_std::*;

/// `ACTOR_EN_IN` (`actor_table.h`: 0x00CB).
pub const ACTOR_EN_IN: i16 = 0x00CB;
pub const OBJECT: &str = "object_in";
/// `OBJECT_IN` (`object_table.h`: 0x00C0).
const OBJECT_IN: i16 = 0x00C0;

/// `En_In_Profile`.
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_EN_IN,
    name: "En_In",
    category: ACTORCAT_NPC,
    flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY | ACTOR_FLAG_UPDATE_CULLING_DISABLED,
    object: OBJECT,
    schema: &SCHEMA,
};

/// `this->actor.params`: which adult Ingo (the race's); over 0 the adult-era body.
pub const TYPE: Field = Field::whole(
    "params",
    Kind::Named(&[
        (-1, "placed: the scene, the age, the time and rot.z decide"),
        (1, "the race's rider (En_Horse's HORSE_PTYPE_INGO_SPAWNED_RIDING, rot.z 1)"),
        (2, "the ranch's adult Ingo offering the rental or the race (race states 0, 2, 3, 4, 7)"),
        (3, "the ranch's adult Ingo in the rental period (race state 1)"),
        (4, "the ranch's adult Ingo trapped by his loss (race states 5, 6)"),
    ]),
    "which Ingo of the adult's race; over 0 the adult-era body (5: set by his init asleep before Epona)",
);
/// `this->actor.shape.rot.z` (`EnIn_GetStartMode`, `EnIn_StartingRace`): which Ingo.
pub const ROT_Z: Field = Field::rot(
    "shape.rot.z",
    Source::RotZ,
    0,
    16,
    Kind::Named(&[
        (1, "the ranch by day, a child before Talon's return (working); the race's rider (params 1)"),
        (2, "the stable by night, a child before Talon's return (working)"),
        (3, "the stable by day, a child after Talon's return (working)"),
        (4, "the stable by night, a child after Talon's return (working)"),
        (5, "the ranch by day, an adult without Epona (the rental and the race)"),
        (6, "Lon Lon's buildings by night, an adult without Epona (asleep)"),
        (7, "the ranch by day, an adult with Epona (obedient)"),
        (8, "Lon Lon's buildings by night, an adult with Epona (asleep)"),
    ]),
    "which Ingo, by the scene, the age, the time and the story",
);

// `EVENTCHKINF_*`, `INFTABLE_*`, `EVENTINF_*` (`save.h`).
pub const EVENTCHKINF_11: u16 = 0x11;
pub const EVENTCHKINF_TALON_RETURNED_FROM_CASTLE: u16 = 0x14;
pub const EVENTCHKINF_EPONA_OBTAINED: u16 = 0x18;
pub const EVENTCHKINF_1B: u16 = 0x1B;
pub const INFTABLE_INGO_TALKED_TO_CHILD_LINK_BEFORE_TALON_RETURNED: u16 = 0x94;
pub const INFTABLE_INGO_TALKED_TO_CHILD_LINK_AFTER_TALON_RETURNED: u16 = 0x97;
pub const INFTABLE_INGO_TALKED_TO_ADULT_LINK: u16 = 0x9A;
pub const INFTABLE_A2: u16 = 0xA2;
pub const INFTABLE_AB: u16 = 0xAB;
/// `EVENTINF_INDEX_HORSES`: the race's word of `eventInf`.
pub const EVENTINF_INDEX_HORSES: usize = 0;
/// `EVENTINF_INGO_RACE_STATE_MASK` (`0xF << 0`).
const EVENTINF_INGO_RACE_STATE_MASK: u16 = 0xF;
pub const EVENTINF_INGO_RACE_HORSETYPE: u16 = 0x04;
pub const EVENTINF_INGO_RACE_LOST_ONCE: u16 = 0x05;
pub const EVENTINF_INGO_RACE_SECOND_RACE: u16 = 0x06;
pub const EVENTINF_INGO_RACE_0F: u16 = 0x0F;

// `IngoRaceState` (`save.h`).
pub const INGO_RACE_STATE_OFFER_RENTAL: u16 = 0;
pub const INGO_RACE_STATE_HORSE_RENTAL_PERIOD: u16 = 1;
pub const INGO_RACE_STATE_RACING: u16 = 2;
pub const INGO_RACE_STATE_PLAYER_LOSE: u16 = 3;
pub const INGO_RACE_STATE_FIRST_WIN: u16 = 4;
pub const INGO_RACE_STATE_TRAPPED_WIN_UNUSED: u16 = 5;
pub const INGO_RACE_STATE_TRAPPED_WIN_EPONA: u16 = 6;
pub const INGO_RACE_STATE_REMATCH: u16 = 7;

/// En_In's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[TYPE, ROT_Z],
    flags: &[
        FlagUse::new(event_chk_inf!(EVENTCHKINF_TALON_RETURNED_FROM_CASTLE)).reads().note("where the child's Ingo works; his texts"),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_EPONA_OBTAINED)).reads().note("where the adult's Ingo is; his texts; asleep's params 5"),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_11)).sets().note("text 0x203F closed"),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_1B)).reads().sets().note("the rental's texts (0x2036 to 0x2038); set by the rental's end").indirect("sets: not ported (the adult's race)"),
        FlagUse::new(inf_table!(INFTABLE_INGO_TALKED_TO_CHILD_LINK_BEFORE_TALON_RETURNED)).reads().sets().note("text 0x203F closed"),
        FlagUse::new(inf_table!(INFTABLE_INGO_TALKED_TO_CHILD_LINK_AFTER_TALON_RETURNED)).reads().sets().note("text 0x2045 closed"),
        FlagUse::new(inf_table!(INFTABLE_INGO_TALKED_TO_ADULT_LINK)).reads().sets().note("0x2030 or 0x2031's choice"),
        FlagUse::new(inf_table!(INFTABLE_A2)).reads().sets().clears().note("0x2038's choice; cleared as the rental starts").indirect("clears: not ported (the adult's race)"),
        FlagUse::new(inf_table!(INFTABLE_AB)).reads().sets().note("riding in the rental period").indirect("not ported (the adult's race)"),
        FlagUse::new(Flag::Other("eventInf[EVENTINF_INDEX_HORSES]'s race state (EVENTINF_INGO_RACE_STATE_MASK)"))
            .reads()
            .sets()
            .clears()
            .note("read by the start and the adult's texts; 0x205B's no sets INGO_RACE_STATE_OFFER_RENTAL; the word cleared by sIsObedient's reset"),
        FlagUse::new(event_inf!(EVENTINF_INGO_RACE_LOST_ONCE)).reads().sets().clears().note("the adult's texts; cleared by 0x205B's no").indirect("sets: not ported (the adult's race)"),
        FlagUse::new(event_inf!(EVENTINF_INGO_RACE_SECOND_RACE)).reads().sets().clears().note("the adult's texts; cleared by 0x205B's no").indirect("sets: not ported (the adult's race)"),
        FlagUse::new(event_inf!(EVENTINF_INGO_RACE_HORSETYPE)).sets().note("the rented horse's type").indirect("not ported (the adult's race)"),
        FlagUse::new(event_inf!(EVENTINF_INGO_RACE_0F)).sets().clears().note("the race started or a transition out").indirect("not ported (the adult's race)"),
    ],
    todo: false,
    objects: &[],
    texts: &[
        TextLine::new(0x203F).when("shape.rot.z", &[1, 2]).cond("Link a child, only at Lon Lon Ranch, the first time").note("I'm Ingo; that lazy bum Talon"),
        TextLine::new(0x2040).when("shape.rot.z", &[1, 2]).cond("Link a child, only at Lon Lon Ranch, once he's been talked to").note("tough to be a working man"),
        TextLine::new(0x2045).when("shape.rot.z", &[3, 4]).cond("Link a child, only in the stable, the first time").note("I do all the work around here"),
        TextLine::new(0x2046).when("shape.rot.z", &[3, 4]).cond("Link a child, only in the stable, once he's been talked to").note("I should be in charge"),
    ],
};

// `SCENE_*` (`scene_table.h`).
pub const SCENE_STABLE: u16 = 0x36;
pub const SCENE_LON_LON_BUILDINGS: u16 = 0x4C;
pub const SCENE_LON_LON_RANCH: u16 = 0x63;

/// `PLAYER_STATE1_23` (`player.h`): on horseback.
const PLAYER_STATE1_23: u32 = 1 << 23;

/// `ATTENTION_RANGE_3`, `ATTENTION_RANGE_6` (`actor.h`).
const ATTENTION_RANGE_3: u8 = 3;
const ATTENTION_RANGE_6: u8 = 6;
/// `NpcTrackingMode` (`actor.h`).
const NPC_TRACKING_PLAYER_AUTO_TURN: i16 = 0;
const NPC_TRACKING_NONE: i16 = 1;
/// `NpcTalkState` (`actor.h`).
pub const NPC_TALK_STATE_IDLE: i16 = 0;
pub const NPC_TALK_STATE_TALKING: i16 = 1;
pub const NPC_TALK_STATE_ACTION: i16 = 2;
/// `FIDGET_FREQ_Y`, `FIDGET_FREQ_Z`, `FIDGET_FREQ_LIMB`, `FIDGET_AMPLITUDE` (`actor.h`).
const FIDGET_FREQ_Y: u32 = 0x814;
const FIDGET_FREQ_Z: u32 = 0x940;
const FIDGET_FREQ_LIMB: u32 = 0x32;
const FIDGET_AMPLITUDE: f32 = 200.0;

// `IngoLimb` (`object_in.xml`), 1-based as `EnIn_OverrideLimbDraw` sees them.
const INGO_LIMB_CHEST: usize = 9;
const INGO_LIMB_LEFT_SHOULDER: usize = 10;
const INGO_LIMB_LEFT_HAND: usize = 12;
const INGO_LIMB_RIGHT_SHOULDER: usize = 13;
const INGO_LIMB_RIGHT_HAND: usize = 15;
const INGO_LIMB_HEAD: usize = 16;
pub const INGO_LIMB_MAX: usize = 20;

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
    dim: Cylinder16 { radius: 18, height: 46, y_shift: 0, pos: [0; 3] },
};

/// `sColChkInfoInit`: immovable.
const COL_CHK_INFO_INIT: CollisionCheckInfoInit2 = CollisionCheckInfoInit2 { health: 0, cyl_radius: 0, cyl_height: 0, cyl_y_shift: 0, mass: MASS_IMMOVABLE };

// `EnInAnimation`: indices into `sAnimationInfo`.
pub const ENIN_ANIM_STANDING: usize = 0;
pub const ENIN_ANIM_STANDING_MORPH: usize = 1;
pub const ENIN_ANIM_STANDING_FOOT_FORWARD: usize = 2;
pub const ENIN_ANIM_STANDING_FOOT_FORWARD_MORPH: usize = 3;
pub const ENIN_ANIM_GLOATING: usize = 4;
pub const ENIN_ANIM_RUNNING: usize = 5;
pub const ENIN_ANIM_LOSING: usize = 6;
pub const ENIN_ANIM_SLEEPING: usize = 7;
pub const ENIN_ANIM_OBEDIENT: usize = 8;
pub const ENIN_ANIM_WORKING: usize = 9;

/// `sAnimationInfo`: the animation, its mode and morph frames (all at speed 1).
const ANIMATION_INFO: [(&str, u8, f32); 10] = [
    ("gIngoStandingAnim", ANIMMODE_LOOP, 0.0),
    ("gIngoStandingAnim", ANIMMODE_LOOP, -10.0),
    ("gIngoStandingFootForwardAnim", ANIMMODE_LOOP, 0.0),
    ("gIngoStandingFootForwardAnim", ANIMMODE_LOOP, -10.0),
    ("gIngoGloatingAnim", ANIMMODE_LOOP, 0.0),
    ("gIngoRunningAnim", ANIMMODE_LOOP, -10.0),
    ("gIngoLosingAnim", ANIMMODE_LOOP, 0.0),
    ("gIngoSleepingAnim", ANIMMODE_LOOP, 0.0),
    ("gIngoObedientAnim", ANIMMODE_LOOP, 0.0),
    ("gIngoWorkingAnim", ANIMMODE_LOOP, 0.0),
];
pub const WORKING_ANIM: &str = "gIngoWorkingAnim";
const RUNNING_ANIM: &str = "gIngoRunningAnim";
const LOSING_ANIM: &str = "gIngoLosingAnim";
const SLEEPING_ANIM: &str = "gIngoSleepingAnim";

// `EnInHorseAnimation` (`z_en_in.h`).
const ENIN_HORSE_ANIM_SITTING_FORWARD: usize = 1;
/// `sHorsebackAnimations`, by `EnInHorseAnimation`.
const HORSEBACK_ANIMATIONS: [&str; 8] = [
    "gIngoHorsebackSlowRideAnim",
    "gIngoHorsebackSittingForwardAnim",
    "gIngoHorsebackRearingAnim",
    "gIngoHorsebackLashingFastAnim",
    "gIngoHorsebackLashingSlowAnim",
    "gIngoHorsebackKickingAnim",
    "gIngoHorsebackSittingUprightAnim",
    "gIngoHorsebackWhoopingAnim",
];

/// `sAdultEraDLs`, by limb (1-based; none for the root, and the head's is never used: `EnIn_OverrideLimbDraw`
/// skips `INGO_LIMB_HEAD`).
const ADULT_ERA_DLS: [Option<&str>; INGO_LIMB_MAX] = [
    None,
    None,
    Some("gIngoAdultEraLeftThighDL"),
    Some("gIngoAdultEraLeftLegDL"),
    Some("gIngoAdultEraLeftFootDL"),
    Some("gIngoAdultEraRightThighDL"),
    Some("gIngoAdultEraRightLegDL"),
    Some("gIngoAdultEraRightFootDL"),
    Some("gIngoAdultEraTorsoDL"),
    Some("gIngoAdultEraChestDL"),
    Some("gIngoAdultEraLeftShoulderDL"),
    Some("gIngoAdultEraLeftArmDL"),
    Some("gIngoAdultEraLeftHandDL"),
    Some("gIngoAdultEraRightShoulderDL"),
    Some("gIngoAdultEraRightArmDL"),
    Some("gIngoAdultEraRightHandDL"),
    Some("gIngoAdultEraHeadDL"),
    Some("gIngoAdultEraLeftEyebrowDL"),
    Some("gIngoAdultEraRightEyebrowDL"),
    Some("gIngoAdultEraMustacheDL"),
];

/// `EnIn_Draw`'s `eyeTextures`.
const EYE_TEXTURES: [&str; 4] = ["gIngoEyeOpenTex", "gIngoEyeHalfTex", "gIngoEyeClosedTex", "gIngoEyeClosed2Tex"];
const SEG_EYES: u8 = 0x08;
const SEG_HEAD_GRADIENT: u8 = 0x09;

/// The skeleton drawn with eye `eye`, in the child-era body or (`adult_era`, `params > 0`) the
/// adult-era one.
pub fn bake_name(eye: usize, adult_era: bool) -> String {
    if adult_era { format!("En_In/adult/eye{eye}") } else { format!("En_In/eye{eye}") }
}
/// `gIngoChildEraBasketDL`, `gIngoChildEraPitchForkDL` (`EnIn_PostLimbDraw`).
pub const BASKET_BAKE: &str = "En_In/basket";
pub const PITCHFORK_BAKE: &str = "En_In/pitchfork";

fn segments(eye: usize) -> Vec<(u8, BakeSegment)> {
    vec![
        (SEG_EYES, BakeSegment::Texture { file: OBJECT.into(), symbol: EYE_TEXTURES[eye].into() }),
        (SEG_HEAD_GRADIENT, BakeSegment::Texture { file: OBJECT.into(), symbol: "gIngoHeadGradient2Tex".into() }),
    ]
}

/// The meshes `EnIn_Draw` draws: the skeleton with each eye in each era, the basket and the
/// pitchfork.
pub fn bakes() -> Vec<MeshBake> {
    let mut v = Vec::new();
    for adult_era in [false, true] {
        for eye in 0..EYE_TEXTURES.len() {
            // EnIn_OverrideLimbDraw: params > 0, every limb but the head takes sAdultEraDLs[limb].
            let limbs = if adult_era {
                (1..INGO_LIMB_MAX)
                    .filter(|&l| l != INGO_LIMB_HEAD)
                    .filter_map(|l| ADULT_ERA_DLS[l].map(|s| LimbOverride { limb: (l - 1) as u8, file: OBJECT.into(), symbol: s.into() }))
                    .collect()
            } else {
                Vec::new()
            };
            v.push(MeshBake {
                name: bake_name(eye, adult_era),
                object: OBJECT.into(),
                segments: segments(eye),
                prelude: Vec::new(),
                body: BakeBody::Skeleton { file: OBJECT.into(), symbol: "gIngoSkel".into(), limbs },
            });
        }
    }
    for (name, dl) in [(BASKET_BAKE, "gIngoChildEraBasketDL"), (PITCHFORK_BAKE, "gIngoChildEraPitchForkDL")] {
        v.push(MeshBake { name: name.into(), object: OBJECT.into(), segments: segments(0), prelude: Vec::new(), body: BakeBody::DLists(vec![(OBJECT.into(), dl.into())]) });
    }
    v
}

// `EnInStartMode`.
pub const ENIN_START_MODE_NULL: i32 = 0;
pub const ENIN_START_MODE_WORKING: i32 = 1;
pub const ENIN_START_MODE_RUNNING_RANCH: i32 = 2;
pub const ENIN_START_MODE_SLEEPING: i32 = 3;
pub const ENIN_START_MODE_OBEDIENT: i32 = 4;

/// The overlay's statics (ADR 0059).
#[derive(Default)]
struct Statics {
    /// `sIsObedient`: the horses' `eventInf` word reset once.
    s_is_obedient: bool,
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    WaitForObject,
    Idle,
    // The adult's race and rental (not ported, Phase 13).
    Racing,
    StartRentalPeriod,
    RentalPeriod,
    OfferingRematch,
    ClosingGate,
    GuardingGate,
    PlayerEscapesRanch,
    EndRentalPeriod,
    FinishClosingGate,
    PlayerLostRace,
    PlayerWonFirstRace,
    PlayerWonSecondRace,
}

pub struct EnIn {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    pub skeleton: Arc<Skeleton>,
    pub action: Action,
    pub collider: ColliderCylinder,
    /// `requiredObjectSlot`.
    pub required_object_slot: Option<usize>,
    pub horseback_animation_idx: i16,
    /// `horsebackAnimFrame`: the race's rider's frame, set by his horse (`EnHorse_UpdateIngoRace`).
    pub horseback_anim_frame: f32,
    pub unused_animation_idx: i16,
    pub blink_timer: i16,
    pub losing_scream_timer: i16,
    pub eye_index: i16,
    pub text_state: i16,
    pub prev_text_state: i16,
    pub unk_1fc: i16,
    pub interact_info: NpcTrack,
    /// `fidgetVec[i].y`, `.z`.
    pub fidget_y: [i16; INGO_LIMB_MAX],
    pub fidget_z: [i16; INGO_LIMB_MAX],
    animations: Vec<Anim>,
    horseback: Vec<Anim>,
    /// `AnimTaskQueue_AddActorMovement` queued this frame (`EnIn_Stagger`).
    movement_queued: bool,
}

/// `En_Horse`'s write to its rider (`EnHorse_UpdateIngoRace`, `EnHorse_SetIngoAnimation`):
/// `horsebackAnimationIdx` and `horsebackAnimFrame`. Nothing for a rider that isn't an `En_In`.
pub fn set_horseback_animation(play: &mut PlayState, rider: oot_game::actor_ctx::ActorHandle, anim: i16, frame: f32) {
    if let Some(i) = play.actors.downcast_mut::<EnIn>(rider) {
        i.horseback_animation_idx = anim;
        i.horseback_anim_frame = frame;
    }
}

/// Link's `stateFlags1`.
fn player_state1(play: &PlayState) -> u32 {
    play.player.and_then(|h| play.actors.get(h)).and_then(|p| p.as_player()).map(|p| p.state_flags1()).unwrap_or(0)
}

/// `GET_EVENTINF_INGO_RACE_STATE()`.
pub fn race_state(play: &PlayState) -> u16 {
    // schema: the race's state, bits of eventInf's horses word.
    play.save.event_inf[EVENTINF_INDEX_HORSES] & EVENTINF_INGO_RACE_STATE_MASK
}

/// `SET_EVENTINF_INGO_RACE_STATE(v)`.
fn set_race_state(play: &mut PlayState, v: u16) {
    // schema: the race's state, bits of eventInf's horses word.
    let w = &mut play.save.event_inf[EVENTINF_INDEX_HORSES];
    *w = (*w & !EVENTINF_INGO_RACE_STATE_MASK) | v;
}

/// `WRITE_EVENTINF_INGO_RACE_0F(v)`.
fn write_race_0f(play: &mut PlayState, v: u16) {
    // schema: EVENTINF_INGO_RACE_0F written as a bit (not ported: the race's).
    let w = &mut play.save.event_inf[EVENTINF_INDEX_HORSES];
    *w = (*w & !(1 << EVENTINF_INGO_RACE_0F)) | (v << EVENTINF_INGO_RACE_0F);
}

impl EnIn {
    /// `EnIn_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let Some(assets) = play.assets.clone() else {
            let mut a = actor;
            a.kill();
            return oot_game::spawn::Placeholder::init(a, play);
        };
        let loaded = (|| -> anyhow::Result<_> {
            let animations = ANIMATION_INFO.iter().map(|(n, _, _)| assets.animation(OBJECT, n)).collect::<anyhow::Result<Vec<_>>>()?;
            let horseback = HORSEBACK_ANIMATIONS.iter().map(|n| assets.animation(OBJECT, n)).collect::<anyhow::Result<Vec<_>>>()?;
            Ok((assets.skeleton(OBJECT, "gIngoSkel")?, animations, horseback))
        })();
        let (skeleton, animations, horseback) = match loaded {
            Ok(x) => x,
            Err(e) => {
                log::error!("En_In: {e:#}");
                let mut a = actor;
                a.kill();
                return oot_game::spawn::Placeholder::init(a, play);
            }
        };
        let mut this = EnIn {
            // SkelAnime_InitFlex(.., gIngoSkel, NULL, jointTable, morphTable, INGO_LIMB_MAX), in
            // EnIn_WaitForObject.
            skel: SkelAnimeStd::init_flex(skeleton.limbs.len(), None),
            skeleton,
            action: Action::WaitForObject,
            collider: ColliderCylinder::new(&CYLINDER_INIT),
            required_object_slot: None,
            horseback_animation_idx: 0,
            horseback_anim_frame: 0.0,
            unused_animation_idx: 0,
            blink_timer: 0,
            losing_scream_timer: 0,
            eye_index: 0,
            text_state: 0,
            prev_text_state: 0,
            unk_1fc: 0,
            interact_info: NpcTrack::default(),
            fidget_y: [0; INGO_LIMB_MAX],
            fidget_z: [0; INGO_LIMB_MAX],
            animations,
            horseback,
            movement_queued: false,
            actor,
        };

        // schema: its own profile object, object_in
        this.required_object_slot = play.object_ctx.get_index(OBJECT_IN);
        if this.required_object_slot.is_none() && TYPE.get(this.actor.params) > 0 {
            this.actor.kill();
            return Box::new(this);
        }
        let respawn_pos = play.save.respawn[RESPAWN_MODE_DOWN].pos;
        // Hardcoded coords for Lon Lon's entrance.
        let statics = play.overlay_static::<Statics>(ACTOR_EN_IN);
        if !statics.s_is_obedient && respawn_pos.x == 1107.0 && respawn_pos.y == 0.0 && respawn_pos.z == -3740.0 {
            statics.s_is_obedient = true;
            // schema: the horses' whole eventInf word cleared.
            play.save.event_inf[EVENTINF_INDEX_HORSES] = 0;
        }
        this.action = Action::WaitForObject;

        // EnIn_WaitForObject's choice, made here: where it would start the adult's race or start
        // modes, a placeholder.
        if let Some(what) = this.adult_part(play) {
            log::info!("En_In: the adult's race and rental (Phase 13) aren't ported: a placeholder ({what})");
            return oot_game::spawn::Placeholder::init(this.actor, play);
        }
        Box::new(this)
    }

    /// The adult's part `EnIn_WaitForObject` would start on the first update: the race
    /// (`EnIn_StartingRace`), `ENIN_START_MODE_RUNNING_RANCH` when he stays, `_SLEEPING` or
    /// `_OBEDIENT`.
    fn adult_part(&self, play: &PlayState) -> Option<&'static str> {
        if self.starting_race_check(play) {
            return Some("EnIn_StartingRace: the race's rider");
        }
        match Self::get_start_mode(&self.actor, play) {
            ENIN_START_MODE_RUNNING_RANCH if Self::stays_spawned(race_state(play), TYPE.get(self.actor.params)) => Some("ENIN_START_MODE_RUNNING_RANCH"),
            ENIN_START_MODE_SLEEPING => Some("ENIN_START_MODE_SLEEPING"),
            ENIN_START_MODE_OBEDIENT => Some("ENIN_START_MODE_OBEDIENT"),
            _ => None,
        }
    }

    /// `EnIn_GetTextIdChild`.
    pub fn get_text_id_child(play: &PlayState) -> u16 {
        let s = &play.save;
        if s.get_event_chk_inf(EVENTCHKINF_TALON_RETURNED_FROM_CASTLE) {
            return if s.get_inf_table(INFTABLE_INGO_TALKED_TO_CHILD_LINK_AFTER_TALON_RETURNED) { 0x2046 } else { 0x2045 };
        }
        if s.get_inf_table(INFTABLE_INGO_TALKED_TO_CHILD_LINK_BEFORE_TALON_RETURNED) { 0x2040 } else { 0x203F }
    }

    /// `EnIn_GetTextIdAdult`.
    pub fn get_text_id_adult(play: &PlayState) -> u16 {
        // MaskReaction_GetTextId(play, MASK_REACTION_SET_INGO): not ported, 0.
        let s = &play.save;
        if s.get_event_chk_inf(EVENTCHKINF_EPONA_OBTAINED) {
            return if s.is_day() { 0x205F } else { 0x2057 }; // schema: not ported (the adult's Ingo)
        }
        if !s.is_day() {
            return 0x204E; // schema: not ported (the adult's Ingo)
        }
        match race_state(play) {
            INGO_RACE_STATE_HORSE_RENTAL_PERIOD => {
                if player_state1(play) & PLAYER_STATE1_23 == 0 {
                    0x2036 // schema: not ported (the adult's Ingo)
                } else if s.get_event_chk_inf(EVENTCHKINF_1B) {
                    if s.get_inf_table(INFTABLE_A2) { 0x2036 } else { 0x2038 } // schema: not ported (the adult's Ingo)
                } else {
                    0x2037 // schema: not ported (the adult's Ingo)
                }
            }
            INGO_RACE_STATE_PLAYER_LOSE => {
                if s.get_event_inf(EVENTINF_INGO_RACE_SECOND_RACE) || s.get_event_inf(EVENTINF_INGO_RACE_LOST_ONCE) {
                    0x203E // schema: not ported (the adult's Ingo)
                } else {
                    0x203D // schema: not ported (the adult's Ingo)
                }
            }
            INGO_RACE_STATE_FIRST_WIN => 0x203A, // schema: not ported (the adult's Ingo)
            INGO_RACE_STATE_TRAPPED_WIN_UNUSED | INGO_RACE_STATE_TRAPPED_WIN_EPONA => 0x203C, // schema: not ported (the adult's Ingo)
            // Ask to race again after losing the race.
            INGO_RACE_STATE_REMATCH => 0x205B, // schema: not ported (the adult's Ingo)
            // INGO_RACE_STATE_RACING and the rest: pay to ride, or his first words first.
            _ => {
                if s.get_inf_table(INFTABLE_INGO_TALKED_TO_ADULT_LINK) {
                    0x2031 // schema: not ported (the adult's Ingo)
                } else {
                    0x2030 // schema: not ported (the adult's Ingo)
                }
            }
        }
    }

    /// `EnIn_GetTextId`.
    pub fn get_text_id(play: &PlayState) -> u16 {
        // MaskReaction_GetTextId(play, MASK_REACTION_SET_INGO): not ported, 0.
        if !play.save.adult { Self::get_text_id_child(play) } else { Self::get_text_id_adult(play) }
    }

    /// `EnIn_UpdateTalkStateOnClosing`.
    fn update_talk_state_on_closing(play: &mut PlayState, actor: &Actor) -> i16 {
        let mut talk_state = NPC_TALK_STATE_IDLE;
        match actor.text_id {
            0x2045 => play.save.set_inf_table(INFTABLE_INGO_TALKED_TO_CHILD_LINK_AFTER_TALON_RETURNED),
            0x203E => talk_state = NPC_TALK_STATE_ACTION, // schema: not ported (the adult's Ingo)
            0x203F => {
                play.save.set_event_chk_inf(EVENTCHKINF_11);
                play.save.set_inf_table(INFTABLE_INGO_TALKED_TO_CHILD_LINK_BEFORE_TALON_RETURNED);
            }
            _ => {}
        }
        talk_state
    }

    /// `EnIn_UpdateTalkStateOnChoice`.
    fn update_talk_state_on_choice(play: &mut PlayState, actor: &mut Actor, action: &mut Action) -> i16 {
        let mut talk_state = NPC_TALK_STATE_TALKING;
        let choice = play.msg_ctx.choice_index;
        match actor.text_id {
            0x2030 | 0x2031 => { // schema: not ported (the adult's Ingo)
                actor.text_id = if choice == 1 {
                    0x2032 // schema: not ported (the adult's Ingo)
                } else if play.save.rupees < 10 {
                    0x2033 // schema: not ported (the adult's Ingo)
                } else {
                    0x2034 // schema: not ported (the adult's Ingo)
                };
                play.continue_textbox(actor.text_id);
                play.save.set_inf_table(INFTABLE_INGO_TALKED_TO_ADULT_LINK);
            }
            0x2034 => { // schema: not ported (the adult's Ingo)
                if choice == 1 {
                    oot_game::item::rupees_change_by(&mut play.save, -10);
                    actor.text_id = 0x205C; // schema: not ported (the adult's Ingo)
                } else {
                    actor.text_id = 0x2035; // schema: not ported (the adult's Ingo)
                }
                play.continue_textbox(actor.text_id);
            }
            0x2036 | 0x2037 => { // schema: not ported (the adult's Ingo)
                if choice == 1 {
                    talk_state = NPC_TALK_STATE_ACTION;
                } else {
                    actor.text_id = 0x201F; // schema: not ported (the adult's Ingo)
                    play.continue_textbox(actor.text_id);
                }
            }
            0x2038 => { // schema: not ported (the adult's Ingo)
                if choice == 0 && play.save.rupees >= 50 {
                    talk_state = NPC_TALK_STATE_ACTION;
                } else {
                    actor.text_id = 0x2039; // schema: not ported (the adult's Ingo)
                    play.continue_textbox(actor.text_id);
                    play.save.set_inf_table(INFTABLE_A2);
                }
            }
            0x205B => { // schema: not ported (the adult's Ingo)
                if choice == 0 && play.save.rupees >= 50 {
                    talk_state = NPC_TALK_STATE_ACTION;
                } else {
                    actor.text_id = 0x2039; // schema: not ported (the adult's Ingo)
                    play.continue_textbox(actor.text_id);
                    set_race_state(play, INGO_RACE_STATE_OFFER_RENTAL);
                    play.save.clear_event_inf(EVENTINF_INGO_RACE_LOST_ONCE);
                    play.save.clear_event_inf(EVENTINF_INGO_RACE_SECOND_RACE);
                    *action = Action::StartRentalPeriod;
                }
            }
            _ => {}
        }
        // `if (!gSaveContext.save.info.playerData.rupees) {}`: nothing.
        talk_state
    }

    /// `EnIn_UpdateTalkStateOnEvent`.
    fn update_talk_state_on_event(play: &mut PlayState, actor: &mut Actor) -> i16 {
        if actor.text_id == 0x2035 { // schema: not ported (the adult's Ingo)
            oot_game::item::rupees_change_by(&mut play.save, -10);
            actor.text_id = 0x205C; // schema: not ported (the adult's Ingo)
            play.continue_textbox(actor.text_id);
            NPC_TALK_STATE_TALKING
        } else {
            NPC_TALK_STATE_ACTION
        }
    }

    /// `EnIn_UpdateTalkState`.
    fn update_talk_state(play: &mut PlayState, actor: &mut Actor, action: &mut Action) -> i16 {
        log::trace!("message_check->({}[{:x}])", play.message_state(), actor.text_id);
        match play.message_state() {
            TEXT_STATE_CLOSING => Self::update_talk_state_on_closing(play, actor),
            TEXT_STATE_CHOICE => {
                if play.message_should_advance() {
                    Self::update_talk_state_on_choice(play, actor, action)
                } else {
                    NPC_TALK_STATE_TALKING
                }
            }
            TEXT_STATE_EVENT => {
                if play.message_should_advance() {
                    Self::update_talk_state_on_event(play, actor)
                } else {
                    NPC_TALK_STATE_TALKING
                }
            }
            _ => NPC_TALK_STATE_TALKING,
        }
    }

    /// `EnIn_UpdateTracking`: Link, from 16 up; not while running, losing or asleep.
    fn update_tracking(&mut self, play: &mut PlayState) {
        let tracking_mode = if self.skel.is(RUNNING_ANIM) || self.skel.is(LOSING_ANIM) || self.skel.is(SLEEPING_ANIM) { NPC_TRACKING_NONE } else { NPC_TRACKING_PLAYER_AUTO_TURN };
        // EnIn_RentalPeriod: NPC_TRACKING_FULL_BODY; EnIn_EndRentalPeriod: the view's eye, from 60
        // up. Neither is ported (the adult's race), so neither is ever his action here.
        self.interact_info.target = play.player.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos).unwrap_or(self.actor.world_pos);
        self.interact_info.eye_height = 16.0;
        let mut t = self.interact_info;
        track_point(play, &mut self.actor, &mut t, 1, tracking_mode);
        self.interact_info = t;
    }

    /// `EnIn_Stagger`: the losing animation's root motion carried over when it rises.
    fn stagger(&mut self) {
        if (self.skel.base_transl[1] as i32) < self.skel.joint_table[0][1] as i32 {
            self.skel.movement_flags |= ANIM_FLAG_UPDATE_XZ | ANIM_FLAG_UPDATE_Y;
            // AnimTaskQueue_AddActorMovement(play, &actor, skelAnime, 1.0f).
            self.movement_queued = true;
        }
    }

    /// `EnIn_ChangeAnim`.
    fn change_anim(&mut self, index: usize) {
        let (_, mode, morph) = ANIMATION_INFO[index];
        let anim = self.animations[index].clone();
        let last = anim.last_frame();
        self.skel.change(anim, 1.0, 0.0, last, mode, morph);
    }

    /// `EnIn_StartingRace`'s check: the race's rider (params 1, rot.z 1) for an adult.
    fn starting_race_check(&self, play: &PlayState) -> bool {
        !(TYPE.get(self.actor.params) != 1 || ROT_Z.get(self.actor.shape_rot.z) != 1 || !play.save.adult)
    }

    /// `EnIn_StartingRace`.
    fn starting_race(&mut self, play: &PlayState) -> bool {
        if !self.starting_race_check(play) {
            return false;
        }
        self.horseback_animation_idx = ENIN_HORSE_ANIM_SITTING_FORWARD as i16;
        self.collider.base.oc_flags1 &= !OC1_ON;
        let anim = self.horseback[self.horseback_animation_idx as usize].clone();
        let last = anim.last_frame();
        self.skel.change(anim, 1.0, 0.0, last, ANIMMODE_ONCE, 0.0);
        self.action = Action::Racing;
        true
    }

    /// `EnIn_GetStartMode`.
    pub fn get_start_mode(actor: &Actor, play: &PlayState) -> i32 {
        let s = &play.save;
        let (child, adult, day, night) = (!s.adult, s.adult, s.is_day(), !s.is_day());
        let rot_z = ROT_Z.get(actor.shape_rot.z);
        let talon_back = s.get_event_chk_inf(EVENTCHKINF_TALON_RETURNED_FROM_CASTLE);
        let epona = s.get_event_chk_inf(EVENTCHKINF_EPONA_OBTAINED);
        let scene = play.scene_id;
        if scene == SCENE_LON_LON_RANCH && child && day && rot_z == 1 && !talon_back {
            return ENIN_START_MODE_WORKING;
        }
        if scene == SCENE_STABLE && child && day && rot_z == 3 && talon_back {
            return ENIN_START_MODE_WORKING;
        }
        if scene == SCENE_STABLE && child && night {
            if rot_z == 2 && !talon_back {
                return ENIN_START_MODE_WORKING;
            }
            if rot_z == 4 && talon_back {
                return ENIN_START_MODE_WORKING;
            }
        }
        if scene == SCENE_LON_LON_RANCH && adult && day {
            if rot_z == 5 && !epona {
                return ENIN_START_MODE_RUNNING_RANCH;
            }
            if rot_z == 7 && epona {
                return ENIN_START_MODE_OBEDIENT;
            }
        }
        if scene == SCENE_LON_LON_BUILDINGS && adult && night {
            if rot_z == 6 && !epona {
                return ENIN_START_MODE_SLEEPING;
            }
            if rot_z == 8 && epona {
                return ENIN_START_MODE_SLEEPING;
            }
        }
        ENIN_START_MODE_NULL
    }

    /// `EnIn_WaitForObject`'s `staySpawned` for `ENIN_START_MODE_RUNNING_RANCH`: the Ingo of
    /// `params` for the race's state.
    fn stays_spawned(race_state: u16, params: i16) -> bool {
        match race_state {
            INGO_RACE_STATE_OFFER_RENTAL | INGO_RACE_STATE_RACING | INGO_RACE_STATE_PLAYER_LOSE | INGO_RACE_STATE_FIRST_WIN | INGO_RACE_STATE_REMATCH => params == 2,
            INGO_RACE_STATE_HORSE_RENTAL_PERIOD => params == 3,
            INGO_RACE_STATE_TRAPPED_WIN_UNUSED | INGO_RACE_STATE_TRAPPED_WIN_EPONA => params == 4,
            _ => false,
        }
    }

    /// `EnIn_UpdateEyes`: blinks, but not with his eyes shut for good (3).
    fn update_eyes(&mut self, play: &mut PlayState) {
        if self.eye_index != 3 {
            // DECR(blinkTimer) == 0.
            if self.blink_timer != 0 {
                self.blink_timer -= 1;
            }
            if self.blink_timer == 0 {
                self.eye_index += 1;
                if self.eye_index >= 3 {
                    self.blink_timer = play.rand.s16_offset(30, 30);
                    self.eye_index = 0;
                }
            }
        }
    }

    /// `EnIn_Fidget`: the sway's angles by the frame; all 0 (still) while working.
    fn fidget(&mut self, play: &PlayState) {
        let f: u32 = if !self.skel.is(WORKING_ANIM) { play.gameplay_frames } else { 0 };
        for i in 0..INGO_LIMB_MAX {
            self.fidget_y[i] = (FIDGET_FREQ_Y + FIDGET_FREQ_LIMB * i as u32).wrapping_mul(f) as i16;
            self.fidget_z[i] = (FIDGET_FREQ_Z + FIDGET_FREQ_LIMB * i as u32).wrapping_mul(f) as i16;
        }
    }

    /// `EnIn_WaitForObject`: `OBJECT_IN` is his profile's, so loaded: his setup by the start mode.
    fn wait_for_object(&mut self, play: &mut PlayState) {
        let loaded = self.required_object_slot.is_some_and(|s| play.object_ctx.is_loaded(s));
        if !(loaded || TYPE.get(self.actor.params) <= 0) {
            return;
        }
        self.actor.shape_init(0.0, oot_game::shadow::ShadowDraw::Circle, 36.0);
        self.collider = ColliderCylinder::new(&CYLINDER_INIT);
        self.actor.col_chk_info.set_info2(None, &COL_CHK_INFO_INIT);
        if self.starting_race(play) {
            write_race_0f(play, 0);
            return;
        }
        self.actor.scale = Vec3::splat(0.01);
        self.actor.target_mode = ATTENTION_RANGE_6;
        self.interact_info.talk_state = NPC_TALK_STATE_IDLE;
        self.action = Action::Idle;

        match Self::get_start_mode(&self.actor, play) {
            ENIN_START_MODE_WORKING => {
                self.change_anim(ENIN_ANIM_WORKING);
                self.action = Action::Idle;
            }
            ENIN_START_MODE_SLEEPING => {
                self.change_anim(ENIN_ANIM_SLEEPING);
                self.action = Action::Idle;
                if !play.save.get_event_chk_inf(EVENTCHKINF_EPONA_OBTAINED) {
                    // schema: the C writes params.
                    self.actor.params = 5;
                }
            }
            ENIN_START_MODE_OBEDIENT => {
                self.change_anim(ENIN_ANIM_OBEDIENT);
                self.eye_index = 3;
                self.action = Action::Idle;
            }
            ENIN_START_MODE_NULL => self.actor.kill(),
            // ENIN_START_MODE_RUNNING_RANCH.
            _ => {
                let state = race_state(play);
                if !Self::stays_spawned(state, TYPE.get(self.actor.params)) {
                    self.actor.kill();
                    return;
                }
                match state {
                    INGO_RACE_STATE_OFFER_RENTAL | INGO_RACE_STATE_RACING => {
                        self.change_anim(ENIN_ANIM_STANDING_FOOT_FORWARD);
                        self.action = Action::StartRentalPeriod;
                        // schema: the horses' whole eventInf word cleared.
                        play.save.event_inf[EVENTINF_INDEX_HORSES] = 0;
                    }
                    INGO_RACE_STATE_HORSE_RENTAL_PERIOD => {
                        self.actor.target_mode = ATTENTION_RANGE_3;
                        self.change_anim(ENIN_ANIM_STANDING_FOOT_FORWARD);
                        self.action = Action::RentalPeriod;
                        play.interface_set_timer(60);
                    }
                    INGO_RACE_STATE_PLAYER_LOSE => {
                        self.change_anim(ENIN_ANIM_GLOATING);
                        self.action = Action::PlayerLostRace;
                    }
                    INGO_RACE_STATE_FIRST_WIN => {
                        self.change_anim(ENIN_ANIM_LOSING);
                        self.losing_scream_timer = 8;
                        self.action = Action::PlayerWonFirstRace;
                    }
                    INGO_RACE_STATE_TRAPPED_WIN_UNUSED | INGO_RACE_STATE_TRAPPED_WIN_EPONA => {
                        self.actor.target_mode = ATTENTION_RANGE_3;
                        self.change_anim(ENIN_ANIM_LOSING);
                        self.losing_scream_timer = 8;
                        self.action = Action::PlayerWonSecondRace;
                    }
                    INGO_RACE_STATE_REMATCH => {
                        self.change_anim(ENIN_ANIM_STANDING_FOOT_FORWARD);
                        self.action = Action::OfferingRematch;
                    }
                    _ => {}
                }
            }
        }
    }

    /// The limb matrices `EnIn_Draw` draws with: `EnIn_OverrideLimbDraw`'s head and chest turns
    /// and the fidget's sway of the chest and shoulders (`sway`: limb, y angle, z angle).
    fn pose(skeleton: &Skeleton, joints: &[[i16; 3]], head: [i16; 2], torso: [i16; 2], sway: &[(usize, i16, i16)]) -> Vec<Mat4> {
        // BINANG_TO_RAD_ALT.
        let r = binang_to_rad;
        skeleton.pose_override(joints, |limb, _pos, rot| {
            let mut pre = Mat4::IDENTITY;
            if limb == INGO_LIMB_HEAD {
                // Translate(1500, 0, 0), RotateZ(headRot.x), RotateX(headRot.y), Translate(-1500, 0, 0).
                pre = Mat4::from_translation(Vec3::new(1500.0, 0.0, 0.0)) * Mat4::from_rotation_z(r(head[0])) * Mat4::from_rotation_x(r(head[1])) * Mat4::from_translation(Vec3::new(-1500.0, 0.0, 0.0));
            }
            if limb == INGO_LIMB_CHEST {
                // RotateX(torsoRot.x), RotateY(torsoRot.y).
                pre = Mat4::from_rotation_x(r(torso[0])) * Mat4::from_rotation_y(r(torso[1]));
            }
            if let Some(&(_, a, b)) = sway.iter().find(|s| s.0 == limb) {
                rot[1] = (rot[1] as f32 + sin_s(a) * FIDGET_AMPLITUDE) as i32 as i16;
                rot[2] = (rot[2] as f32 + cos_s(b) * FIDGET_AMPLITUDE) as i32 as i16;
            }
            pre
        })
    }

    fn sway_of(y: &[i16; INGO_LIMB_MAX], z: &[i16; INGO_LIMB_MAX]) -> [(usize, i16, i16); 3] {
        [INGO_LIMB_CHEST, INGO_LIMB_LEFT_SHOULDER, INGO_LIMB_RIGHT_SHOULDER].map(|l| (l, y[l], z[l]))
    }
}

/// Indices into the render state's extras.
mod rs {
    /// `angles`: head x, y; torso x, y; the sway (chest, left and right shoulders: y, z each).
    pub const HEAD: usize = 0;
    pub const TORSO: usize = 2;
    pub const SWAY: usize = 4;
    pub const ANGLES: usize = 10;
    /// `switches`: the eye, the adult-era body, working (the basket and pitchfork).
    pub const EYE: usize = 0;
    pub const ADULT_ERA: usize = 1;
    pub const WORKING: usize = 2;
}

impl ActorImpl for EnIn {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `EnIn_Update`.
    fn update(&mut self, play: &mut PlayState) {
        if self.action == Action::WaitForObject {
            self.wait_for_object(play);
            return;
        }
        self.collider.update(&self.actor);
        play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
        if self.action != Action::Racing {
            self.skel.update();
            if self.skel.is(LOSING_ANIM) && race_state(play) != INGO_RACE_STATE_TRAPPED_WIN_EPONA {
                self.stagger();
            }
            self.actor.update_bg_check_info(&play.col, 0.0, 0.0, 0.0, UPDBGCHECKINFO_FLAG_2);
        }
        self.update_eyes(play);
        match self.action {
            Action::Idle => {}
            Action::WaitForObject => {}
            a => log::debug!("En_In: EnIn_{a:?} (the adult's race and rental, Phase 13) isn't ported"),
        }
        if self.action != Action::Racing {
            self.fidget(play);
            let s = &play.save;
            if s.sub_timer_seconds < 6 && s.sub_timer_state != SUBTIMER_STATE_OFF && self.interact_info.talk_state == NPC_TALK_STATE_IDLE {
                // `if (Actor_TalkOfferAccepted(&this->actor, play)) {}`.
                process_talk_request(&mut self.actor);
            } else {
                // Npc_UpdateTalking(.., (attentionRangeType == 6 ? 80 : 320) + collider.dim.radius,
                // EnIn_GetTextId, EnIn_UpdateTalkState).
                let range = if self.actor.target_mode == ATTENTION_RANGE_6 { 80.0 } else { 320.0 } + self.collider.dim.radius as f32;
                let mut talk_state = self.interact_info.talk_state;
                let action = &mut self.action;
                talk_update(play, &mut self.actor, &mut talk_state, range, |play, _| Self::get_text_id(play), |play, actor| Self::update_talk_state(play, actor, action));
                self.interact_info.talk_state = talk_state;
                if self.interact_info.talk_state != NPC_TALK_STATE_IDLE {
                    self.prev_text_state = self.text_state;
                    self.text_state = play.message_state() as i16;
                }
            }
            self.update_tracking(play);
        }
    }
    /// `AnimTask_ActorMovement` for `EnIn_Stagger`'s queued move: the root's motion, at his scale.
    fn animation_update(&mut self) {
        if !std::mem::take(&mut self.movement_queued) {
            return;
        }
        let diff = self.skel.update_translation(self.actor.shape_rot.y);
        self.actor.world_pos.x += diff.x * self.actor.scale.x;
        self.actor.world_pos.y += diff.y * self.actor.scale.y;
        self.actor.world_pos.z += diff.z * self.actor.scale.z;
    }
    /// `EnIn_PostLimbDraw`, the head: the focus 1600 along it, `focus.rot` the world's.
    fn draw_update(&mut self, _play: &mut PlayState) {
        if self.action == Action::WaitForObject {
            return;
        }
        let t = &self.interact_info;
        let bones = Self::pose(&self.skeleton, &self.skel.joint_table, [t.head[0], t.head[1]], [t.torso[0], t.torso[1]], &Self::sway_of(&self.fidget_y, &self.fidget_z));
        if let Some(head) = bones.get(INGO_LIMB_HEAD - 1) {
            let m = oot_game::play::actor_draw_matrix(&RenderState::of(&self.actor));
            self.actor.focus_pos = (m * *head).transform_point3(Vec3::new(1600.0, 0.0, 0.0));
            self.actor.focus_rot = self.actor.world_rot;
        }
    }
    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        // EnIn_Draw: nothing while EnIn_WaitForObject.
        if self.action == Action::WaitForObject {
            return rs;
        }
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        let t = &self.interact_info;
        let sway = Self::sway_of(&self.fidget_y, &self.fidget_z);
        rs.angles = vec![t.head[0], t.head[1], t.torso[0], t.torso[1], sway[0].1, sway[0].2, sway[1].1, sway[1].2, sway[2].1, sway[2].2];
        rs.switches = vec![self.eye_index.clamp(0, 3) as u32, (TYPE.get(self.actor.params) > 0) as u32, self.skel.is(WORKING_ANIM) as u32];
        rs
    }
    /// `EnIn_Draw`: `Gfx_SetupDL_25Opa`, the eyes on 8, `gIngoHeadGradient2Tex` on 9, the
    /// skeleton; `EnIn_PostLimbDraw`'s basket and pitchfork.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let Some(joints) = &rs.joints else { return };
        if rs.angles.len() < rs::ANGLES || rs.switches.len() < 3 {
            return;
        }
        let a = &rs.angles;
        let sway = [
            (INGO_LIMB_CHEST, a[rs::SWAY], a[rs::SWAY + 1]),
            (INGO_LIMB_LEFT_SHOULDER, a[rs::SWAY + 2], a[rs::SWAY + 3]),
            (INGO_LIMB_RIGHT_SHOULDER, a[rs::SWAY + 4], a[rs::SWAY + 5]),
        ];
        let bones = Self::pose(&self.skeleton, &joints.rot, [a[rs::HEAD], a[rs::HEAD + 1]], [a[rs::TORSO], a[rs::TORSO + 1]], &sway);
        let m = oot_game::play::actor_draw_matrix(rs);
        let eye = (rs.switches[rs::EYE] as usize).min(EYE_TEXTURES.len() - 1);
        if rs.switches[rs::WORKING] != 0 {
            for (limb, bake) in [(INGO_LIMB_LEFT_HAND, BASKET_BAKE), (INGO_LIMB_RIGHT_HAND, PITCHFORK_BAKE)] {
                if let Some(b) = bones.get(limb - 1) {
                    out.opa.push(DrawCmd::new(MeshKey::named(keys::bake(bake)), m * *b));
                }
            }
        }
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(&bake_name(eye, rs.switches[rs::ADULT_ERA] != 0))), transform: m, bones, params: Default::default() });
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
    fn the_adult_era_body_keeps_the_head() {
        let b = bakes();
        let adult = b.iter().find(|m| m.name == bake_name(0, true)).unwrap();
        let BakeBody::Skeleton { limbs, .. } = &adult.body else { panic!() };
        // Limbs 2 to 19 but the head (16): 17 lists.
        assert_eq!(limbs.len(), 17);
        assert!(limbs.iter().all(|l| l.limb as usize != INGO_LIMB_HEAD - 1));
        assert_eq!(b.len(), 10);
    }

    #[test]
    fn rental_states_keep_their_ingo() {
        assert!(EnIn::stays_spawned(INGO_RACE_STATE_OFFER_RENTAL, 2));
        assert!(!EnIn::stays_spawned(INGO_RACE_STATE_OFFER_RENTAL, -1));
        assert!(EnIn::stays_spawned(INGO_RACE_STATE_HORSE_RENTAL_PERIOD, 3));
        assert!(EnIn::stays_spawned(INGO_RACE_STATE_TRAPPED_WIN_EPONA, 4));
        assert!(!EnIn::stays_spawned(INGO_RACE_STATE_REMATCH, 4));
    }
}
