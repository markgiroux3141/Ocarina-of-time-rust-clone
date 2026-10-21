//! `En_Ru1` (`ovl_En_Ru1/z_en_ru1.c`, `z_en_ru1_cutscene_data.inc.c`): Ruto as a child (GAME-10
//! milestone 5), of `object_ru1`, by type (`params & 0xFF`):
//!
//! - **The boss room** (type 0, `ENRU1_TYPE_BOSS_ROOM`; Barinade's, spawned by `Boss_Va`): hands on
//!   her hips, blushing, mouth open, 10000 under the floor (`shape.yOffset`); the next frame she
//!   turns to Link and spawns her blue warp as her child (`Door_Warp1` `WARP_BLUE_RUTO`, 12 towards
//!   him) and rises through it (250/3 a frame); up, she sets its `rutoWarpState` ready. When the
//!   warp says Link entered (`WARP_BLUE_RUTO_STATE_ENTERED`) she marks his spot 30 in front of her
//!   (`unk_450`), and once he stands still there (3 frames on) he's turned to face her; then "what
//!   took you so long" (`WARP_BLUE_RUTO_STATE_3`: arms and legs crossed, then sitting while the
//!   warp talks), and when the warp warps (`WARP_BLUE_RUTO_STATE_WARPING`) she rises with Link, at
//!   her distance along his facing. The warp's own Ruto states (`DoorWarp1_RutoWarpIdle`,
//!   `_RutoWarpOut`, `EVENTCHKINF_37`) are a later milestone's.
//! - **Zora's Fountain** (type 1, `gZorasFountainSapphireCs`'s cues on channel 3): gazing at Link
//!   (hands behind her back, at cue 2's place; a giggle on frame 130, Link's splash on 205); past
//!   cue 2 a dive (hidden, a splash and three ripples at the surface, `NA_SE_EV_DIVE_INTO_WATER`);
//!   past cue 3 resurfacing (`gRutoChildResurfaceAnim`, `NA_SE_PL_FACE_UP` on frame 5), then
//!   treading water towards Link along the cue (a cosine ease), a ripple each 10 frames; cue 6 she
//!   turns on her back (`NA_SE_PL_SUBMERGE` on frame 8); past it she swims away on her back from
//!   where she trod (`NA_SE_PL_SWIM` four times a stroke) and two frames before the cue's end turns
//!   back; on cue 8 with that done she's gone.
//! - **The holes room** (type 2, Jabu-Jabu's room 2): not met (`INFTABLE_RUTO_MET_FIRST_TIME`), she
//!   waits frowning (hidden) until Link is within 265 along z of her, not below her, on the
//!   ground, not climbing or hanging and with no cutscene: `gRutoFirstMeetingCs` (Link stopped).
//!   Its cues (channel 3): past 1 she stands at the cue facing Link (its texts 0x404B and 0x401E
//!   are the script's); cue 3 she turns round and walks away (2.7 a frame after 8 frames' speed-up,
//!   `NA_SE_PL_WALK_GROUND` on Jabu's surface on frames 2 and 7); off the floor she drops down the
//!   hole (13 a frame after 8 frames' speed-up); cue 4 her arms go up with `NA_SE_EV_FALL_DOWN_DIRT`
//!   and her cry (`NA_SE_VO_RT_FALL`); cue 5 sets `INFTABLE_RUTO_MET_FIRST_TIME` and once the script
//!   ends she's gone. Brought back here (`INFTABLE_RUTO_BROUGHT_BACK_TO_HOLES_ROOM`, neither on the
//!   switch nor with the sapphire) she stands to be talked to as in the basement, unless another
//!   Ruto is carried or waiting.
//! - **The basement** (type 3, room 3; met, neither on the switch, with the sapphire nor brought
//!   back, and no other Ruto carried or waiting): she follows rooms (`room` -1), her head and torso
//!   on Link (`Npc_TrackPoint`, preset 12). Talked to: 0x404C, then (`INFTABLE_RUTO_EXPLAINED_WHY_IN_JABU`)
//!   0x404D, then (`INFTABLE_RUTO_LET_LINK_CARRY`) 0x404E; turned to him while talking. As 0x404D's
//!   box closes she sits (`gRutoChildSitAnim`, the box paused, `NA_SE_EV_LAND_DIRT` on frame 11)
//!   and, seated, sets `INFTABLE_RUTO_LET_LINK_CARRY` and asks to be carried (`Actor_OfferCarry`).
//!   Sitting she has a cylinder of 20 by 30 (its OC on once Link is 32 away); lifted (her parent
//!   set: `NA_SE_PL_PULL_UP_RUTO`, `NA_SE_VO_RT_LIFT`) she looks about while carried; Link idling
//!   with her (`PLAYER_STATE2_IDLE_FIDGET`) 30 frames, she may squirm (a 1 in 3 each 30 frames,
//!   `NA_SE_VO_RT_UNBALLANCE`) for 50. Put down or thrown (`NA_SE_VO_RT_THROW` if thrown; gravity
//!   -1.3, at most 6.8 a frame down, her cylinder hurting as `AT_TYPE_PLAYER`): off a scene floor or
//!   ceiling at 0.8 of her speed, bouncing at 0.6 of her fall, off walls at 0.6 of her speed
//!   (`NA_SE_VO_RT_CRASH`), `NA_SE_EV_LAND_DIRT` each time; slowed 0.98 a frame; a bg actor's floor
//!   stops her dead but a chest's (`ACTOR_EN_BOX`) bounces her back up and speeds her (its floor
//!   flags dropped each frame). Stopped on the floor she sits again; in water she's slowed (0.2 of
//!   her fall, half her speed each frame), bobs (the half of her depth less 10, 0.95 a frame), then
//!   sinks (gravity -0.1, 0.7 at most) to the bottom and sits there, or, in a room she wasn't
//!   placed in and over 50 deep, fades out over 20 frames and is gone. In a room she wasn't let go
//!   in, she's gone. Carried, she reads her floor: on a blue `Bg_Bdan_Switch` with flag 0x38 she
//!   sets `INFTABLE_RUTO_PLACED_ON_SWITCH`, else clears it (Master Quest's room 3 switch is 0x3C:
//!   it never sets); carried into room 2 she sets `INFTABLE_RUTO_BROUGHT_BACK_TO_HOLES_ROOM`. She
//!   presses switches herself (`ACTOR_FLAG_CAN_PRESS_SWITCHES`: a blue one holds while she sits on
//!   it).
//! - **The sapphire** (the basement's Ruto in room 6): carried where the Zora's Sapphire
//!   (`Demo_Effect` `DEMO_EFFECT_JEWEL_ZORA`) is, not yet brought (`INFTABLE_RUTO_BROUGHT_TO_SAPPHIRE_ROOM`)
//!   and no cutscene: `gRutoFoundSapphireCs`, she squirms (`NA_SE_VO_RT_DISCOVER`) and when it
//!   ends sets the flag. Landing (thrown or let go) on Big Octo's platform (`Bg_Bdan_Objects` type
//!   0) with no cutscene or box: `gRutoObtainingSapphireCs` (hidden until past cue 1): she walks to
//!   the sapphire and holds it up (root motion; her laugh on frame 5, eyes up at frame 60 with
//!   `NA_SE_VO_RT_THROW`, steps), past cue 2 brings her hands down; the script over (not on a
//!   cutscene layer) `INFTABLE_RUTO_HAS_SAPPHIRE`, her switch flag and the platform's
//!   `CAM_SET_NORMAL0`: it rises (at `CAM_SET_NORMAL1` she looks up and shudders,
//!   `NA_SE_VO_RT_THROW`); at `CAM_SET_DUNGEON0` she says 0x4048 and cries as Big Octo takes her
//!   (`SfxSource_PlaySfxAtFixedWorldPos`), hidden; the box closing, `CAM_SET_DUNGEON1` and she's
//!   gone. Its texts 0x4050 to 0x4052 are the script's.
//! - **The sapphire room** (type 4, room 6; with the sapphire, not abducted): waits hidden for
//!   the platform, then as above from `CAM_SET_NORMAL0`.
//! - **Beside King Zora** (type 5, Zora's Domain; `EVENTCHKINF_37` and Link a child): talks
//!   (0x402C, or the mask's line, `MASK_REACTION_SET_RUTO`), her head on Link, turned to him
//!   while talking.
//! - **Beside the door switch** (type 6, room 7; met, on the switch, without the sapphire, no
//!   other Ruto carried or waiting): as the basement's.
//! - **Debug** (type 10, `DEBUG_FEATURES`): straight to the platform's cutscene states (hidden).
//!
//! Drawn by `drawConfig`: nothing, opaque, or translucent at `alpha` (`EnRu1_DrawXlu`, as she
//! fades); her eyes on segment 8 (`sEyeTextures`: open, half, closed, rolled, half again, blushing)
//! and her mouth on 9 (closed, frowning, open; the C binds the eyes there first); her chest and
//! head turned by the tracking (`RUTO_CHILD_CHEST`, `_HEAD`), her focus 10 above her head (the
//! opaque draw's). Her blink every 60 to 120 frames. A circle shadow (30).
//!
//! Placed in this ROM: Jabu-Jabu (Master Quest) layers 0 to 3: 0x0202 (room 2), 0x0203 (room 3),
//! 0x0004 (room 6), 0x0206 (room 7, which doesn't load `object_ru1`); layer 4: 0x0002, 0x0003,
//! 0x0004; Zora's Fountain layer 4: 0x0001; Zora's Domain layers 0 and 1: 0x0005. Barinade
//! (`Boss_Va`) spawns type 0.
//!
//! Not ported here: Big Octo (`En_Bigokuta`, milestone 8: what she does to it is the platform's,
//! which spawns it); the warp's Ruto states; Player's own Ruto checks (the main session's).
//!
//! Params: `type` bits 0 to 7 (`EnRu1Type`: 0 the boss room, 1 Zora's Fountain, 2 the holes room,
//! 3 the basement, 4 the sapphire room, 5 beside King Zora, 6 beside the door switch, 10 debug;
//! any other kills her at init); `switchFlag` bits 8 to 15 (set as she takes the sapphire).
//! Flags: reads and sets `INFTABLE_RUTO_MET_FIRST_TIME` (the meeting), `_EXPLAINED_WHY_IN_JABU`,
//! `_LET_LINK_CARRY`, `_BROUGHT_TO_SAPPHIRE_ROOM`, `_HAS_SAPPHIRE`, `_BROUGHT_BACK_TO_HOLES_ROOM`;
//! reads, sets and clears `INFTABLE_RUTO_PLACED_ON_SWITCH`; reads `INFTABLE_RUTO_ABDUCTED` and
//! `EVENTCHKINF_37` (King Zora's Ruto); sets switch flag `switchFlag`.

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::bgcheck::BGCHECK_SCENE;
use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, DrawImage, DrawParams, MeshKey, SegmentValues, SourceImage};
use eng_math::{cos_s, sin_s, smooth_step_to_s};
use glam::{Mat4, Vec3};
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_BG, ACTORCAT_BOSS, ACTORCAT_NPC, ActorHandle, ActorImpl, ActorProfile, dyna_poly_get_actor};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind, TextLine};
use oot_game::audio::sfx::{NA_SE_EV_DIVE_INTO_WATER, NA_SE_PL_FACE_UP, NA_SE_PL_WALK_GROUND, NA_SE_VO_RT_THROW, SfxPos};
use oot_game::camera::{CAM_SET_DUNGEON0, CAM_SET_NONE, CAM_SET_NORMAL0};
use oot_game::collision_check::*;
use oot_game::cutscene::{CS_STATE_IDLE, CsCmdActorCue};
use oot_game::gbi::Dl;
use oot_game::message::{MSGMODE_PAUSED, TEXT_STATE_CLOSING, TEXT_STATE_EVENT};
use oot_game::npc::{NpcTrack, offer_talk_default, process_talk_request, track_point};
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::skelanime_std::*;
use oot_game::{event_chk_inf, inf_table};

use crate::bg_bdan_objects::{ACTOR_BG_BDAN_OBJECTS, BgBdanObjects, CAM_SET_DUNGEON1, CAM_SET_NORMAL1, INFTABLE_RUTO_ABDUCTED};
use crate::bg_bdan_switch::ACTOR_BG_BDAN_SWITCH;
use crate::boss_goma::ACTOR_DOOR_WARP1;
use crate::demo_effect::{ACTOR_DEMO_EFFECT, DEMO_EFFECT_JEWEL_ZORA};
use crate::door_warp1::{DoorWarp1, WARP_BLUE_RUTO, WARP_BLUE_RUTO_STATE_3, WARP_BLUE_RUTO_STATE_ENTERED, WARP_BLUE_RUTO_STATE_READY, WARP_BLUE_RUTO_STATE_TALKING, WARP_BLUE_RUTO_STATE_WARPING};
use crate::en_box::ACTOR_EN_BOX;
use crate::player::{Player, STATE1_13, STATE1_14, STATE1_21, STATE2_28};

/// `ACTOR_EN_RU1` (`actor_table.h`: 0x00A1).
pub const ACTOR_EN_RU1: i16 = 0x00A1;
/// `OBJECT_RU1` (`object_table.h`: 0x00A3).
pub const OBJECT: &str = "object_ru1";

/// `En_Ru1_Profile`: `ACTORCAT_NPC`, `ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_UPDATE_CULLING_DISABLED |
/// ACTOR_FLAG_CAN_PRESS_SWITCHES`, `OBJECT_RU1`.
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_EN_RU1,
    name: "En_Ru1",
    category: ACTORCAT_NPC,
    flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_CAN_PRESS_SWITCHES,
    object: OBJECT,
    schema: &SCHEMA,
};

/// `EnRu1Type` (`z_en_ru1.h`).
pub const ENRU1_TYPE_BOSS_ROOM: i16 = 0;
pub const ENRU1_TYPE_FOUNTAIN: i16 = 1;
pub const ENRU1_TYPE_HOLES_ROOM: i16 = 2;
pub const ENRU1_TYPE_BASEMENT: i16 = 3;
pub const ENRU1_TYPE_SAPPHIRE_ROOM: i16 = 4;
pub const ENRU1_TYPE_BESIDE_KZ: i16 = 5;
pub const ENRU1_TYPE_BESIDE_DOOR_SWITCH: i16 = 6;
pub const ENRU1_TYPE_DEBUG: i16 = 10;

/// `ENRU1_TYPE(thisx)`: `PARAMS_GET_U(params, 0, 8)`.
pub const TYPE: Field = Field::params(
    "type",
    0,
    8,
    Kind::Named(&[
        (0, "Barinade's room: rises through her blue warp (Boss_Va spawns her)"),
        (1, "Zora's Fountain: the sapphire's cutscene (cues on channel 3)"),
        (2, "Jabu-Jabu's holes room: the first meeting; later, brought back, to be talked to"),
        (3, "Jabu-Jabu's basement: to be talked to and carried"),
        (4, "Jabu-Jabu's sapphire room: with the sapphire, waiting for Big Octo's platform"),
        (5, "Zora's Domain, beside King Zora (EVENTCHKINF_37, Link a child)"),
        (6, "beside the door switch: once placed on the switch"),
        (10, "debug: straight to the platform's cutscene states"),
    ]),
    "which of child Ruto's places (any other kills her)",
);
/// `ENRU1_SWITCH_FLAG(thisx)`: `PARAMS_GET_U(params, 8, 8)`.
pub const SWITCH_FLAG: Field = Field::params("switchFlag", 8, 8, Kind::SwitchFlag, "set as she takes the sapphire (gRutoObtainingSapphireCs's end)");

/// The infTable flags (`save.h`).
pub const INFTABLE_RUTO_PLACED_ON_SWITCH: u16 = 0x140;
pub const INFTABLE_RUTO_MET_FIRST_TIME: u16 = 0x141;
pub const INFTABLE_RUTO_EXPLAINED_WHY_IN_JABU: u16 = 0x142;
pub const INFTABLE_RUTO_LET_LINK_CARRY: u16 = 0x143;
pub const INFTABLE_RUTO_BROUGHT_TO_SAPPHIRE_ROOM: u16 = 0x144;
pub const INFTABLE_RUTO_HAS_SAPPHIRE: u16 = 0x145;
pub const INFTABLE_RUTO_BROUGHT_BACK_TO_HOLES_ROOM: u16 = 0x147;
/// `EVENTCHKINF_37` (`save.h`: 0x37): set by Ruto's blue warp out of Barinade's room.
pub const EVENTCHKINF_37: u16 = 0x37;

/// En_Ru1's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[TYPE, SWITCH_FLAG],
    flags: &[
        FlagUse::new(inf_table!(INFTABLE_RUTO_MET_FIRST_TIME)).reads().sets().note("the holes room's meeting (cue 5); the basement, the door switch need it"),
        FlagUse::new(inf_table!(INFTABLE_RUTO_EXPLAINED_WHY_IN_JABU)).reads().sets().note("her first talk's close; then 0x404D, and she sits"),
        FlagUse::new(inf_table!(INFTABLE_RUTO_LET_LINK_CARRY)).reads().sets().note("seated: carried; then 0x404E"),
        FlagUse::new(inf_table!(INFTABLE_RUTO_PLACED_ON_SWITCH)).reads().sets().clears().note("carried or let go: on a blue switch with flag 0x38 it's set, else cleared"),
        FlagUse::new(inf_table!(INFTABLE_RUTO_BROUGHT_TO_SAPPHIRE_ROOM)).reads().sets().note("gRutoFoundSapphireCs's end"),
        FlagUse::new(inf_table!(INFTABLE_RUTO_HAS_SAPPHIRE)).reads().sets().note("gRutoObtainingSapphireCs's end"),
        FlagUse::new(inf_table!(INFTABLE_RUTO_BROUGHT_BACK_TO_HOLES_ROOM)).reads().sets().note("carried into room 2"),
        FlagUse::new(inf_table!(INFTABLE_RUTO_ABDUCTED)).reads().note("the sapphire room's Ruto: gone once Big Octo took her"),
        FlagUse::new(event_chk_inf!(EVENTCHKINF_37)).reads().note("beside King Zora: after her blue warp"),
        FlagUse::new(Flag::Switch("switchFlag")).sets().note("as she takes the sapphire"),
    ],
    todo: false,
    objects: &[],
    texts: &[
        TextLine::new(0x404C).when("type", &[2, 3, 6]).cond("in Jabu-Jabu, before she's explained").note("why she's in Jabu-Jabu"),
        TextLine::new(0x404D).when("type", &[2, 3, 6]).cond("explained (INFTABLE_RUTO_EXPLAINED_WHY_IN_JABU): as it closes she sits to be carried"),
        TextLine::new(0x404E).when("type", &[2, 3, 6]).cond("once she's let Link carry her (INFTABLE_RUTO_LET_LINK_CARRY)"),
        TextLine::new(0x4048).when("type", &[3, 4]).cond("on Big Octo's platform as it comes up: Big Octo takes her"),
        TextLine::new(0x402C).when("type", &[5]).cond("beside King Zora, no mask on"),
        TextLine::new(0x7190).when("type", &[5]).cond("beside King Zora, the Keaton Mask, Skull Mask or Gerudo Mask on (MASK_REACTION_SET_RUTO)"),
        TextLine::new(0x7191).when("type", &[5]).cond("beside King Zora, the Spooky Mask, Goron Mask or Mask of Truth on"),
        TextLine::new(0x7192).when("type", &[5]).cond("beside King Zora, the Bunny Hood on"),
        TextLine::new(0x7193).when("type", &[5]).cond("beside King Zora, the Zora Mask on"),
    ],
};

/// `EnRu1Action` (`z_en_ru1.h`): `sActionFuncs`' indices.
pub const ENRU1_ACTION_FOUNTAIN_GAZING_AT_LINK: i32 = 0;
pub const ENRU1_ACTION_FOUNTAIN_DIVING: i32 = 1;
pub const ENRU1_ACTION_FOUNTAIN_RESURFACING: i32 = 2;
pub const ENRU1_ACTION_FOUNTAIN_TREADING_WATER: i32 = 3;
pub const ENRU1_ACTION_FOUNTAIN_STARTING_SWIM_BACK: i32 = 4;
pub const ENRU1_ACTION_FOUNTAIN_SWIMMING_BACK: i32 = 5;
pub const ENRU1_ACTION_FOUNTAIN_FINISHING_SWIM_BACK: i32 = 6;
pub const ENRU1_ACTION_MEETING_RANGE_CHECK: i32 = 7;
pub const ENRU1_ACTION_MEETING_INIT_POSITION: i32 = 8;
pub const ENRU1_ACTION_MEETING_FACING_LINK: i32 = 9;
pub const ENRU1_ACTION_MEETING_TURNING_AROUND: i32 = 10;
pub const ENRU1_ACTION_MEETING_WALKING_AWAY_ACCEL: i32 = 11;
pub const ENRU1_ACTION_MEETING_WALKING_AWAY_CONSTANT: i32 = 12;
pub const ENRU1_ACTION_MEETING_FALLING_DOWN_HOLE: i32 = 13;
pub const ENRU1_ACTION_MEETING_END: i32 = 14;
pub const ENRU1_ACTION_PRE_SPAWN_BOSS_ROOM: i32 = 15;
pub const ENRU1_ACTION_SPAWN_BOSS_ROOM: i32 = 16;
pub const ENRU1_ACTION_RISE_THROUGH_BLUE_WARP: i32 = 17;
pub const ENRU1_ACTION_WAIT_INSIDE_BLUE_WARP: i32 = 18;
pub const ENRU1_ACTION_LINK_WALKS_TO_POINT_IN_BLUE_WARP: i32 = 19;
pub const ENRU1_ACTION_WHAT_TOOK_YOU_SO_LONG: i32 = 20;
pub const ENRU1_ACTION_WARPING_OUT: i32 = 21;
pub const ENRU1_ACTION_SPEAK_JABU_PRE_SPAWN: i32 = 22;
pub const ENRU1_ACTION_SPEAK_JABU_SPAWN: i32 = 23;
pub const ENRU1_ACTION_SPEAK_JABU_IDLE: i32 = 24;
pub const ENRU1_ACTION_SPEAK_JABU_TALKING: i32 = 25;
pub const ENRU1_ACTION_SPEAK_JABU_SIT_DOWN: i32 = 26;
pub const ENRU1_ACTION_SITTING_IDLE: i32 = 27;
pub const ENRU1_ACTION_SITTING_RELEASED: i32 = 28;
pub const ENRU1_ACTION_SITTING_ENTERING_WATER: i32 = 29;
pub const ENRU1_ACTION_SITTING_SINKING: i32 = 30;
pub const ENRU1_ACTION_SITTING_CARRIED: i32 = 31;
pub const ENRU1_ACTION_SITTING_LOSING_BALANCE: i32 = 32;
pub const ENRU1_ACTION_SITTING_DISAPPEARING: i32 = 33;
pub const ENRU1_ACTION_SITTING_SEES_SAPPHIRE: i32 = 34;
pub const ENRU1_ACTION_UNUSED: i32 = 35;
pub const ENRU1_ACTION_THROWN_ONTO_SAPPHIRE_PLATFORM: i32 = 36;
pub const ENRU1_ACTION_RETRIEVING_SAPPHIRE: i32 = 37;
pub const ENRU1_ACTION_HOLDING_SAPPHIRE: i32 = 38;
pub const ENRU1_ACTION_DONE_HOLDING_SAPPHIRE: i32 = 39;
pub const ENRU1_ACTION_MISSION_ACCOMPLISHED: i32 = 40;
pub const ENRU1_ACTION_WAITING_IN_SAPPHIRE_ROOM: i32 = 41;
pub const ENRU1_ACTION_READY_TO_GO_HOME: i32 = 42;
pub const ENRU1_ACTION_BEING_ABDUCTED: i32 = 43;
pub const ENRU1_ACTION_THRONE_IDLE: i32 = 44;
pub const ENRU1_ACTION_THRONE_TALK: i32 = 45;
/// `ARRAY_COUNT(sActionFuncs)`.
const ACTION_FUNCS: i32 = 46;

/// `EnRu1DrawConfig`.
pub const ENRU1_DRAW_NOTHING: i32 = 0;
pub const ENRU1_DRAW_OPA: i32 = 1;
pub const ENRU1_DRAW_XLU: i32 = 2;

/// `EnRu1Eyes`.
pub const ENRU1_EYES_OPEN: i16 = 0;
pub const ENRU1_EYES_UP: i16 = 3;
pub const ENRU1_EYES_GAZING: i16 = 4;
pub const ENRU1_EYES_BLUSH: i16 = 5;
/// `EnRu1Mouth`.
pub const ENRU1_MOUTH_SMILING: i16 = 0;
pub const ENRU1_MOUTH_FROWNING: i16 = 1;
pub const ENRU1_MOUTH_OPEN: i16 = 2;

/// `EnRu1WaterState`.
pub const ENRU1_WATER_OUTSIDE: i32 = 0;
pub const ENRU1_WATER_IMMERSED: i32 = 1;
pub const ENRU1_WATER_BOBBING: i32 = 2;
pub const ENRU1_WATER_SINKING: i32 = 3;

/// `RUTO_CHILD_CHEST`, `RUTO_CHILD_HEAD` (`RutoLimb`, 1-based).
pub const RUTO_CHILD_CHEST: usize = 8;
pub const RUTO_CHILD_HEAD: usize = 15;

/// `CS_STATE_IDLE` aside, the debug type's params (`this->actor.params == 0xA`).
const DEBUG_PARAMS: i16 = 0xA;

// The sounds (`include/tables/sfx/*.h`).
/// `NA_SE_PL_SWIM` (`playerbank_table.h`: 0x839).
pub const NA_SE_PL_SWIM: u16 = 0x0839;
/// `NA_SE_PL_SUBMERGE` (`playerbank_table.h`: 0x873).
pub const NA_SE_PL_SUBMERGE: u16 = 0x0873;
/// `NA_SE_PL_PULL_UP_RUTO` (`playerbank_table.h`: 0x883).
pub const NA_SE_PL_PULL_UP_RUTO: u16 = 0x0883;
/// `NA_SE_EV_FALL_DOWN_DIRT` (`environmentbank_table.h`: 0x2870).
pub const NA_SE_EV_FALL_DOWN_DIRT: u16 = 0x2870;
/// `NA_SE_EV_LAND_DIRT` (`environmentbank_table.h`: 0x28AC).
pub const NA_SE_EV_LAND_DIRT: u16 = 0x28AC;
/// `NA_SE_VO_RT_CRASH` .. `NA_SE_VO_RT_UNBALLANCE` (`voicebank_table.h`: 0x6860 to 0x6866).
pub const NA_SE_VO_RT_CRASH: u16 = 0x6860;
pub const NA_SE_VO_RT_DISCOVER: u16 = 0x6861;
pub const NA_SE_VO_RT_FALL: u16 = 0x6862;
pub const NA_SE_VO_RT_LAUGH_0: u16 = 0x6863;
pub const NA_SE_VO_RT_LIFT: u16 = 0x6864;
pub const NA_SE_VO_RT_UNBALLANCE: u16 = 0x6866;
/// `SURFACE_SFX_OFFSET_JABU` (`bgcheck.h`: 3).
const SURFACE_SFX_OFFSET_JABU: u16 = 3;

/// `NPC_TRACKING_HEAD_AND_TORSO`, `NPC_TRACKING_FULL_BODY` (`actor.h`).
const NPC_TRACKING_HEAD_AND_TORSO: i16 = 2;
const NPC_TRACKING_FULL_BODY: i16 = 4;

/// `MASK_REACTION_SET_RUTO` (`face_reaction.h`: 0x1F).
const MASK_REACTION_SET_RUTO: usize = 0x1F;
/// `sMaskReactionSetTextIds[MASK_REACTION_SET_RUTO]` (`z_face_reaction.c`), by `PLAYER_MASK_*`.
const MASK_REACTION_RUTO: [u16; 9] = [0x0000, 0x7190, 0x7190, 0x7191, 0x7192, 0x7191, 0x7193, 0x7190, 0x7191];

/// The debug registers `kREG(n)` (0: they start zeroed).
const KREG: i32 = 0;

/// `sEyeTextures`, `sMouthTextures`.
pub const EYE_TEXTURES: [&str; 6] = ["gRutoChildEyeOpenTex", "gRutoChildEyeHalfTex", "gRutoChildEyeClosedTex", "gRutoChildEyeRollLeftTex", "gRutoChildEyeHalf2Tex", "gRutoChildEyeHalfWithBlushTex"];
pub const MOUTH_TEXTURES: [&str; 3] = ["gRutoChildMouthClosedTex", "gRutoChildMouthFrownTex", "gRutoChildMouthOpenTex"];
/// The segments the draws bind: the eyes (8), the mouth (9; the C binds the eyes there first, then
/// the mouth over them); a draw swaps the textures in by these addresses.
const SEG_EYES: u32 = 0x0800_0000;
const SEG_MOUTH: u32 = 0x0900_0000;
/// Segment 0xC: `ACTOR_SETUP_OPA_DL` (an end) or `gActorSetupXluDL`.
const SEG_SETUP: u8 = 0x0C;
/// `gDPSetEnvColor(0, 0, 0, alpha)`, a dynamic colour run before the skeleton.
const SEG_ALPHA: u8 = 0x0E;

/// `EnRu1_DrawOpa`'s and `EnRu1_DrawXlu`'s meshes.
pub const BAKE_OPA: &str = "En_Ru1/opa";
pub const BAKE_XLU: &str = "En_Ru1/xlu";

/// `sMultVec`: her focus from her head.
const FOCUS_OFFSET: Vec3 = Vec3::new(0.0, 10.0, 0.0);

// shared: gActorSetupXluDL (z_actor.c), as demo_sa.rs and demo_du.rs build it.
/// `gActorSetupXluDL`: `gsDPSetRenderMode(G_RM_FOG_SHADE_A, G_RM_AA_ZB_XLU_SURF2 | Z_UPD)`,
/// `gsDPSetAlphaCompare(G_AC_THRESHOLD)`, the end.
fn actor_setup_xlu_dl() -> Vec<(u32, u32)> {
    let mut d = Dl::default();
    d.0.push((0xE200_001C, 0xC810_49F8));
    d.othermode_l(0, 2, 1);
    d.end();
    d.0
}

/// Her two meshes: `gRutoChildSkel` with the open eyes on 8, the closed mouth on 9 (swapped per
/// draw), `ACTOR_SETUP_OPA_DL` or `gActorSetupXluDL` on 0xC, the env alpha.
pub fn bakes() -> Vec<MeshBake> {
    let tex = |s: &str| BakeSegment::Texture { file: OBJECT.into(), symbol: s.into() };
    [(BAKE_OPA, false), (BAKE_XLU, true)]
        .into_iter()
        .map(|(name, xlu)| MeshBake {
            name: name.into(),
            object: OBJECT.into(),
            segments: vec![
                (0x08, tex(EYE_TEXTURES[0])),
                (0x09, tex(MOUTH_TEXTURES[0])),
                (SEG_SETUP, BakeSegment::Commands(if xlu { actor_setup_xlu_dl() } else { vec![(0xDF00_0000, 0)] })),
                (SEG_ALPHA, BakeSegment::DynamicColor { env: true, prim: false }),
            ],
            prelude: vec![SEG_ALPHA],
            body: BakeBody::Skeleton { file: OBJECT.into(), symbol: "gRutoChildSkel".into(), limbs: Vec::new() },
        })
        .collect()
}

/// `sStandingCylinderInit` (`ColliderCylinderInitType1`: `OC2_TYPE_1`).
const STANDING_CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_HIT0, at_flags: AT_NONE, ac_flags: AC_NONE, oc_flags1: OC1_ON | OC1_TYPE_PLAYER, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_NONE,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 25, height: 80, y_shift: 0, pos: [0; 3] },
};

/// `sSittingCylinderInit`: thrown, she hurts as Player's attack (`0x00000101`).
const SITTING_CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_HIT0, at_flags: AT_ON | AT_TYPE_PLAYER, ac_flags: AC_NONE, oc_flags1: OC1_ON | OC1_TYPE_PLAYER, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0x0000_0101, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
        at_elem_flags: ATELEM_ON | ATELEM_SFX_NORMAL,
        ac_elem_flags: ACELEM_NONE,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 20, height: 30, y_shift: 0, pos: [0; 3] },
};

/// The colliders' ids (`collider_mut`).
pub const COLLIDER_STANDING: u8 = 0;
pub const COLLIDER_SITTING: u8 = 1;

/// The overlay's statics.
#[derive(Debug, Default)]
pub struct Statics {
    /// `sReachedForSapphire`.
    pub reached_for_sapphire: u32,
}

/// Her animations (`object_ru1.xml`).
struct Anims {
    wait_hands_behind_back: Anim,
    wait_hands_on_hips: Anim,
    transition_hands_on_hip_to_cross_arms_and_legs: Anim,
    wait_sitting: Anim,
    wait_in_blue_warp: Anim,
    bring_hands_down: Anim,
    hold_arms_up: Anim,
    bring_arms_up: Anim,
    turn_around: Anim,
    squirm: Anim,
    sees_sapphire: Anim,
    shutter: Anim,
    walk_to_and_hold_up_sapphire: Anim,
    sitting: Anim,
    sit: Anim,
    walk: Anim,
    transition_from_swim_on_back: Anim,
    transition_to_swim_on_back: Anim,
    resurface: Anim,
    wait2: Anim,
    swim_on_back: Anim,
    tread_water: Anim,
}

pub struct EnRu1 {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    pub skeleton: Arc<Skeleton>,
    pub eyes: i16,
    pub blink_timer: i16,
    pub mouth: i16,
    /// `action`: the index into `sActionFuncs`.
    pub action: i32,
    /// `drawConfig`: the index into `sDrawFuncs`.
    pub draw_config: i32,
    pub walking_frame: f32,
    pub tread_timer: f32,
    /// `blueWarp`: her `Door_Warp1` (`WARP_BLUE_RUTO`).
    pub blue_warp: Option<ActorHandle>,
    pub xz_dist_to_player_in_blue_warp: f32,
    pub is_falling: i32,
    pub room_num1: i8,
    pub room_num2: i8,
    pub room_num3: i8,
    pub unk_288: f32,
    /// `bigOctoPlatform`: `Bg_Bdan_Objects` type 0.
    pub big_octo_platform: Option<ActorHandle>,
    pub pre_limb_draw_index: i32,
    pub unk_298: i32,
    pub head_turn_speed: i16,
    pub sink_timer: f32,
    pub alpha: i32,
    pub head_rot_timer: i16,
    pub head_rot_direction: i32,
    pub standing_collider: ColliderCylinder,
    pub sitting_collider: ColliderCylinder,
    pub is_sitting_oc_active: bool,
    pub water_state: i32,
    pub sinking_start_pos_y: f32,
    pub bob_depth: f32,
    pub bob_phase: i16,
    pub is_sinking: f32,
    pub tread_start_pos: Vec3,
    pub carry_idle_timer: f32,
    pub interact_info: NpcTrack,
    anims: Anims,
    /// The face textures as images: `EYE_TEXTURES`, then `MOUTH_TEXTURES`.
    face: Vec<DrawImage>,
    /// Root motion queued this frame (`AnimTaskQueue_AddActorMovement`).
    movement_queued: bool,
    logged: bool,
}

/// `DECR`: 0 when it was 0, else the decremented value.
fn decr(x: &mut i16) -> i16 {
    if *x == 0 {
        0
    } else {
        *x -= 1;
        *x
    }
}

fn player(play: &PlayState) -> Option<&Player> {
    play.player.and_then(|h| play.actors.downcast::<Player>(h))
}

fn player_pos(play: &PlayState) -> Vec3 {
    play.player.and_then(|h| play.actors.actor(h)).map_or(Vec3::ZERO, |a| a.world_pos)
}

/// `Sfx_PlaySfxAtPos(&this->actor.projectedPos, sfx)`: at the updating actor.
fn sfx(play: &mut PlayState, id: u16) {
    let pos = play.cur_actor.map(SfxPos::Actor).unwrap_or(SfxPos::Default);
    play.audio.play_sfx_at_pos(pos, id);
}

/// `SFX_PLAY_AT_POS(&player->actor.projectedPos, sfx)`: at Link.
fn sfx_at_player(play: &mut PlayState, id: u16) {
    let pos = play.player.map(SfxPos::Actor).unwrap_or(SfxPos::Default);
    play.audio.play_sfx_at_pos(pos, id);
}

// shared: MaskReaction_GetTextId (z_face_reaction.c), as en_du.rs and en_diving_game.rs have it.
/// `MaskReaction_GetTextId`: `sMaskReactionSetTextIds[set][Player_GetMask]`, here Ruto's row.
fn mask_reaction_get_text_id(play: &PlayState, set: usize) -> u16 {
    let mask = player(play).map_or(0, |p| p.current_mask) as usize;
    match set {
        MASK_REACTION_SET_RUTO => MASK_REACTION_RUTO.get(mask).copied().unwrap_or(0),
        _ => 0,
    }
}

/// `CsCmdActorCue`'s start position.
fn cue_start(cue: &CsCmdActorCue) -> Vec3 {
    Vec3::new(cue.start_pos.x as f32, cue.start_pos.y as f32, cue.start_pos.z as f32)
}

impl EnRu1 {
    /// `EnRu1_Init`: the circle shadow (30), `gRutoChildSkel`, both cylinders, then the type's init.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut actor = actor;
        let Some(assets) = play.assets.clone() else {
            actor.kill();
            return oot_game::spawn::Placeholder::init(actor, play);
        };
        let loaded = (|| -> anyhow::Result<_> {
            let skeleton = assets.skeleton(OBJECT, "gRutoChildSkel")?;
            let anim = |s: &str| assets.animation(OBJECT, s);
            let anims = Anims {
                wait_hands_behind_back: anim("gRutoChildWaitHandsBehindBackAnim")?,
                wait_hands_on_hips: anim("gRutoChildWaitHandsOnHipsAnim")?,
                transition_hands_on_hip_to_cross_arms_and_legs: anim("gRutoChildTransitionHandsOnHipToCrossArmsAndLegsAnim")?,
                wait_sitting: anim("gRutoChildWaitSittingAnim")?,
                wait_in_blue_warp: anim("gRutoChildWaitInBlueWarpAnim")?,
                bring_hands_down: anim("gRutoChildBringHandsDownAnim")?,
                hold_arms_up: anim("gRutoChildHoldArmsUpAnim")?,
                bring_arms_up: anim("gRutoChildBringArmsUpAnim")?,
                turn_around: anim("gRutoChildTurnAroundAnim")?,
                squirm: anim("gRutoChildSquirmAnim")?,
                sees_sapphire: anim("gRutoChildSeesSapphireAnim")?,
                shutter: anim("gRutoChildShutterAnim")?,
                walk_to_and_hold_up_sapphire: anim("gRutoChildWalkToAndHoldUpSapphireAnim")?,
                sitting: anim("gRutoChildSittingAnim")?,
                sit: anim("gRutoChildSitAnim")?,
                walk: anim("gRutoChildWalkAnim")?,
                transition_from_swim_on_back: anim("gRutoChildTransitionFromSwimOnBackAnim")?,
                transition_to_swim_on_back: anim("gRutoChildTransitionToSwimOnBackAnim")?,
                resurface: anim("gRutoChildResurfaceAnim")?,
                wait2: anim("gRutoChildWait2Anim")?,
                swim_on_back: anim("gRutoChildSwimOnBackAnim")?,
                tread_water: anim("gRutoChildTreadWaterAnim")?,
            };
            let mut face = Vec::new();
            for s in EYE_TEXTURES.iter().chain(MOUTH_TEXTURES.iter()) {
                let t = assets.pack.texture(OBJECT, s)?;
                face.push(DrawImage { width: t.width, height: t.height, rgba: t.rgba.clone().into() });
            }
            Ok((skeleton, anims, face))
        })();
        let (skeleton, anims, face) = match loaded {
            Ok(x) => x,
            Err(e) => {
                log::error!("En_Ru1: {e:#}");
                actor.kill();
                return oot_game::spawn::Placeholder::init(actor, play);
            }
        };
        let mut this = EnRu1 {
            // SkelAnime_InitFlex(.., &gRutoChildSkel, NULL, jointTable, morphTable, 17).
            skel: SkelAnimeStd::init_flex(skeleton.limbs.len(), None),
            skeleton,
            eyes: 0,
            blink_timer: 0,
            mouth: 0,
            action: 0,
            draw_config: 0,
            walking_frame: 0.0,
            tread_timer: 0.0,
            blue_warp: None,
            xz_dist_to_player_in_blue_warp: 0.0,
            is_falling: 0,
            room_num1: 0,
            room_num2: 0,
            room_num3: 0,
            unk_288: 0.0,
            big_octo_platform: None,
            pre_limb_draw_index: 0,
            unk_298: 0,
            head_turn_speed: 0,
            sink_timer: 0.0,
            alpha: 0,
            head_rot_timer: 0,
            head_rot_direction: 0,
            // EnRu1_InitColliders.
            standing_collider: ColliderCylinder::new(&STANDING_CYLINDER_INIT),
            sitting_collider: ColliderCylinder::new(&SITTING_CYLINDER_INIT),
            is_sitting_oc_active: false,
            water_state: 0,
            sinking_start_pos_y: 0.0,
            bob_depth: 0.0,
            bob_phase: 0,
            is_sinking: 0.0,
            tread_start_pos: Vec3::ZERO,
            carry_idle_timer: 0.0,
            interact_info: NpcTrack::default(),
            anims,
            face,
            movement_queued: false,
            logged: false,
            actor,
        };
        this.actor.shape_init(0.0, oot_game::shadow::ShadowDraw::Circle, 30.0);
        this.standing_collider.update(&this.actor);
        this.sitting_collider.update(&this.actor);
        match this.get_type() {
            ENRU1_TYPE_BOSS_ROOM => this.init_in_boss_room(),
            ENRU1_TYPE_FOUNTAIN => this.init_outside_jabu_jabu(),
            ENRU1_TYPE_HOLES_ROOM => this.init_in_jabu_jabu_holes_room(play),
            ENRU1_TYPE_BASEMENT => this.init_in_jabu_jabu_basement(play),
            ENRU1_TYPE_SAPPHIRE_ROOM => this.init_in_sapphire_room(play),
            ENRU1_TYPE_BESIDE_KZ => this.init_beside_king_zora(play),
            ENRU1_TYPE_BESIDE_DOOR_SWITCH => this.init_beside_door_switch(play),
            ENRU1_TYPE_DEBUG => this.func_80af0050(play),
            t => {
                this.actor.kill();
                log::debug!("En_Ru1: Relevant arge_data = {t} unacceptable");
            }
        }
        Box::new(this)
    }

    /// `EnRu1_GetSwitchFlag`.
    fn get_switch_flag(&self) -> u8 {
        SWITCH_FLAG.get(self.actor.params) as u8
    }

    /// `EnRu1_GetType`.
    fn get_type(&self) -> i16 {
        TYPE.get(self.actor.params)
    }

    fn cur_room(play: &PlayState) -> i8 {
        play.room_ctx.cur.num
    }

    // Colliders.

    /// `EnRu1_UpdateStandingOC`.
    fn update_standing_oc(&mut self, play: &mut PlayState) {
        self.standing_collider.update(&self.actor);
        play.collision_check_set_oc(&self.actor, COLLIDER_STANDING, &mut self.standing_collider);
    }

    /// `EnRu1_UpdateSittingOC`: OC once Link has been more than 32 away.
    fn update_sitting_oc(&mut self, play: &mut PlayState) {
        self.sitting_collider.update(&self.actor);
        if self.is_sitting_oc_active {
            play.collision_check_set_oc(&self.actor, COLLIDER_SITTING, &mut self.sitting_collider);
        } else if self.actor.xz_dist_to_player > 32.0 {
            self.is_sitting_oc_active = true;
        }
    }

    /// `EnRu1_UpdateSittingAT`.
    fn update_sitting_at(&mut self, play: &mut PlayState) {
        self.sitting_collider.update(&self.actor);
        play.collision_check_set_at(&self.actor, COLLIDER_SITTING, &mut self.sitting_collider);
    }

    /// `EnRu1_DisableSittingOC`.
    fn disable_sitting_oc(&mut self) {
        self.is_sitting_oc_active = false;
    }

    // Shared.

    /// `EnRu1_UpdateEyes`: the blink, every 60 to 120 frames.
    fn update_eyes(&mut self, play: &mut PlayState) {
        if decr(&mut self.blink_timer) == 0 {
            self.blink_timer = play.rand.s16_offset(60, 60);
        }
        self.eyes = self.blink_timer;
        if self.eyes >= ENRU1_EYES_UP {
            self.eyes = ENRU1_EYES_OPEN;
        }
    }

    /// `EnRu1_SetEyes`.
    fn set_eyes(&mut self, eyes: i16) {
        self.eyes = eyes;
    }

    /// `EnRu1_SetMouth`.
    fn set_mouth(&mut self, mouth: i16) {
        self.mouth = mouth;
    }

    /// `EnRu1_UpdateBgCheckInfo`: walls (19 up, 25 round), ceiling (30) and floor, falling at 4
    /// for the check.
    fn update_bg_check_info(&mut self, play: &PlayState) {
        let velocity_y_held = self.actor.velocity.y;
        self.actor.velocity.y = -4.0;
        self.actor.update_bg_check_info(&play.col, 19.0, 25.0, 30.0, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_1 | UPDBGCHECKINFO_FLAG_2);
        self.actor.velocity.y = velocity_y_held;
    }

    /// `EnRu1_IsCsStateIdle`.
    fn is_cs_state_idle(play: &PlayState) -> bool {
        play.cs_ctx.state == CS_STATE_IDLE
    }

    /// `EnRu1_GetCue`: the cue on `channel` while a script runs.
    pub fn get_cue(play: &PlayState, channel: usize) -> Option<CsCmdActorCue> {
        if !Self::is_cs_state_idle(play) { play.cs_ctx.npc_actions[channel] } else { None }
    }

    /// `EnRu1_CheckCueMatchingId`.
    fn check_cue_matching_id(play: &PlayState, cue_id: u16, channel: usize) -> bool {
        Self::get_cue(play, channel).is_some_and(|c| c.action == cue_id)
    }

    /// `EnRu1_CheckCueNotMatchingId`.
    fn check_cue_not_matching_id(play: &PlayState, cue_id: u16, channel: usize) -> bool {
        Self::get_cue(play, channel).is_some_and(|c| c.action != cue_id)
    }

    /// `EnRu1_IsAssistingLink`: another Ruto carried, losing her balance or waiting to talk.
    fn is_assisting_link(&self, play: &PlayState) -> bool {
        // (`someEnRu1 != this`: in her init and her update she's out of the arena.)
        play.actors
            .category(ACTORCAT_NPC)
            .iter()
            .filter_map(|&h| play.actors.downcast::<EnRu1>(h))
            .any(|r| r.action == ENRU1_ACTION_SITTING_CARRIED || r.action == ENRU1_ACTION_SITTING_LOSING_BALANCE || r.action == ENRU1_ACTION_SPEAK_JABU_IDLE)
    }

    /// `EnRu1_FindBigOctoPlatform`: the first `Bg_Bdan_Objects` with params 0.
    fn find_big_octo_platform(play: &PlayState) -> Option<ActorHandle> {
        let found = play.actors.category(ACTORCAT_BG).iter().copied().find(|&h| {
            // schema: another actor's params read whole (Bg_Bdan_Objects', cut to its type at its init)
            play.actors.actor(h).is_some_and(|a| a.id == ACTOR_BG_BDAN_OBJECTS && a.params == 0)
        });
        if found.is_none() {
            log::debug!("En_Ru1: There is no stand!!!!!!!!!!!!!!!!!!!!!!!!!");
        }
        found
    }

    fn platform_mut<'a>(&self, play: &'a mut PlayState) -> Option<&'a mut BgBdanObjects> {
        self.big_octo_platform.and_then(|h| play.actors.downcast_mut::<BgBdanObjects>(h))
    }

    /// `EnRu1_SetPlatformCamSetting`.
    fn set_platform_cam_setting(&self, play: &mut PlayState, camera_setting: i16) {
        if let Some(p) = self.platform_mut(play) {
            p.camera_setting = camera_setting;
        }
    }

    /// `EnRu1_GetPlatformCamSetting`.
    fn get_platform_cam_setting(&self, play: &PlayState) -> i16 {
        self.big_octo_platform.and_then(|h| play.actors.downcast::<BgBdanObjects>(h)).map_or(CAM_SET_NONE, |p| p.camera_setting)
    }

    /// `func_80AEB124`: the Zora's Sapphire (`Demo_Effect` `DEMO_EFFECT_JEWEL_ZORA`, a boss).
    fn find_zora_jewel(play: &PlayState) -> Option<ActorHandle> {
        play.actors.category(ACTORCAT_BOSS).iter().copied().find(|&h| {
            // schema: another actor's params read whole (Demo_Effect's type, PARAMS_GET_U(params, 0, 8))
            play.actors.actor(h).is_some_and(|a| a.id == ACTOR_DEMO_EFFECT && (a.params & 0xFF) as u8 == DEMO_EFFECT_JEWEL_ZORA)
        })
    }

    /// `EnRu1_MessageShouldAdvance`.
    fn message_should_advance(play: &mut PlayState) -> bool {
        play.message_state() == TEXT_STATE_EVENT && play.message_should_advance()
    }

    /// `EnRu1_IsTextboxClosing`.
    fn is_textbox_closing(play: &PlayState) -> bool {
        play.message_state() == TEXT_STATE_CLOSING
    }

    /// `func_80AEB1D8` (`DEBUG_FEATURES`): to the platform's cutscene states, hidden and still.
    fn func_80aeb1d8(&mut self, play: &mut PlayState) {
        self.action = ENRU1_ACTION_THROWN_ONTO_SAPPHIRE_PLATFORM;
        self.draw_config = ENRU1_DRAW_NOTHING;
        self.actor.velocity = Vec3::ZERO;
        self.actor.speed_xz = 0.0;
        self.actor.gravity = 0.0;
        self.actor.min_velocity_y = 0.0;
        self.set_platform_cam_setting(play, CAM_SET_NONE);
    }

    /// `func_80AEB220` (`DEBUG_FEATURES`): the debug type, no script.
    fn func_80aeb220(&mut self, play: &mut PlayState) {
        // schema: the whole params compared, as the C does (the debug type's 0xA)
        if Self::is_cs_state_idle(play) && self.actor.params == DEBUG_PARAMS {
            self.func_80aeb1d8(play);
        }
    }

    /// `EnRu1_AnimationChange`: from the start (or, reversed, the end) at speed 1.
    fn animation_change(&mut self, animation: Anim, mode: u8, morph_frames: f32, play_reversed: bool) {
        let frame_count = animation.last_frame();
        let (start_frame, end_frame, playback_speed) = if !play_reversed { (0.0, frame_count, 1.0) } else { (frame_count, 0.0, -1.0) };
        self.skel.change(animation, playback_speed, start_frame, end_frame, mode, morph_frames);
    }

    /// `Animation_Change(&skelAnime, anim, 1, 0, Animation_GetLastFrame(anim), mode, morph)`.
    fn change(&mut self, animation: Anim, mode: u8, morph_frames: f32) {
        let end = animation.last_frame();
        self.skel.change(animation, 1.0, 0.0, end, mode, morph_frames);
    }

    /// `EnRu1_UpdateSkelAnime` (both branches `SkelAnime_Update`).
    fn update_skel_anime(&mut self) -> bool {
        self.skel.update()
    }

    /// `EnRu1_WalkTowardSapphire`: her root's motion queued.
    fn walk_toward_sapphire(&mut self) {
        self.skel.movement_flags |= ANIM_FLAG_UPDATE_XZ;
        self.movement_queued = true;
    }

    /// `EnRu1_StartWalkingTowardSapphire`.
    fn start_walking_toward_sapphire(&mut self) {
        self.skel.movement_flags |= ANIM_FLAG_UPDATE_XZ;
        self.walk_toward_sapphire();
    }

    /// `EnRu1_StopWalkingTowardSapphire`.
    fn stop_walking_toward_sapphire(&mut self) {
        self.skel.movement_flags &= !ANIM_FLAG_UPDATE_XZ;
    }

    // Type 1: Zora's Fountain.

    /// `EnRu1_InitOutsideJabuJabu`.
    fn init_outside_jabu_jabu(&mut self) {
        self.animation_change(self.anims.wait_hands_behind_back.clone(), ANIMMODE_LOOP, 0.0, false);
        self.action = ENRU1_ACTION_FOUNTAIN_GAZING_AT_LINK;
        self.draw_config = ENRU1_DRAW_OPA;
        self.set_eyes(ENRU1_EYES_GAZING);
        self.set_mouth(ENRU1_MOUTH_SMILING);
    }

    /// `EnRu1_GetFountainCue`: channel 3.
    fn get_fountain_cue(play: &PlayState) -> Option<CsCmdActorCue> {
        Self::get_cue(play, 3)
    }

    /// `EnRu1_CheckFountainCueMatchingId`.
    fn check_fountain_cue_matching_id(play: &PlayState, cue_id: u16) -> bool {
        Self::check_cue_matching_id(play, cue_id, 3)
    }

    /// `EnRu1_CheckFountainCueNotMatchingId`.
    fn check_fountain_cue_not_matching_id(play: &PlayState, cue_id: u16) -> bool {
        Self::check_cue_not_matching_id(play, cue_id, 3)
    }

    /// `EnRu1_SpawnRipple`: at the surface above her.
    fn spawn_ripple(&self, play: &mut PlayState, radius_max: i16, life: i16) {
        let pos = Vec3::new(self.actor.world_pos.x, self.actor.world_pos.y + self.actor.y_dist_to_water, self.actor.world_pos.z);
        play.with_ss(|s| s.g_ripple_spawn(pos, 100, radius_max, life));
    }

    /// `EnRu1_SpawnRippleTreading`: one each 10 frames.
    fn spawn_ripple_treading(&mut self, play: &mut PlayState) {
        self.tread_timer += 1.0;
        if self.tread_timer >= KREG as f32 + 10.0 {
            self.spawn_ripple(play, (KREG + 500) as i16, 0);
            self.tread_timer = 0.0;
        }
    }

    /// `EnRu1_SpawnThreeRipples`.
    fn spawn_three_ripples(&self, play: &mut PlayState) {
        self.spawn_ripple(play, (KREG + 500) as i16, 0);
        self.spawn_ripple(play, (KREG + 500) as i16, (KREG as f32 + 10.0) as i16);
        self.spawn_ripple(play, (KREG + 500) as i16, ((KREG as f32 + 10.0) * 2.0) as i16);
    }

    /// `EnRu1_SpawnSplash`.
    fn spawn_splash(&self, play: &mut PlayState) {
        let pos = Vec3::new(self.actor.world_pos.x, self.actor.world_pos.y + self.actor.y_dist_to_water, self.actor.world_pos.z);
        play.with_ss(|s| s.g_splash_spawn(pos, None, None, 1, 0));
    }

    /// `EnRu1_StartMovingInWater`: her root's motion while it rises.
    fn start_moving_in_water(&mut self) {
        if self.skel.base_transl[1] < self.skel.joint_table[0][1] {
            self.skel.movement_flags |= ANIM_FLAG_UPDATE_XZ | ANIM_FLAG_UPDATE_Y;
            self.movement_queued = true;
        }
    }

    /// `EnRu1_MoveInWater`: the root's base and last positions where it is.
    fn move_in_water(&mut self) {
        let root = self.skel.joint_table[0];
        self.skel.base_transl = root;
        self.skel.prev_transl = root;
        if self.skel.base_transl[1] < self.skel.joint_table[0][1] {
            self.skel.movement_flags |= ANIM_FLAG_UPDATE_XZ | ANIM_FLAG_UPDATE_Y;
            self.movement_queued = true;
        }
    }

    /// `EnRu1_StopMovingInWater`.
    fn stop_moving_in_water(&mut self) {
        self.skel.movement_flags &= !(ANIM_FLAG_UPDATE_XZ | ANIM_FLAG_UPDATE_Y);
    }

    /// `EnRu1_GetSwimLerpFactor`: a cosine ease over the cue.
    fn get_swim_lerp_factor(cue: &CsCmdActorCue, play: &PlayState) -> f32 {
        let cs_cur_frame = play.cs_ctx.frames as i32;
        let (start, end) = (cue.start_frame as i32, cue.end_frame as i32);
        if cs_cur_frame < end && end - start > 0 {
            return (cos_s((((cs_cur_frame - start) as f32 / (end - start) as f32) * 32768.0) as i32 as i16) * -0.5) + 0.5;
        }
        1.0
    }

    /// `EnRu1_Lerp`.
    fn lerp(scale: f32, start_pos: i32, end_pos: i32) -> f32 {
        ((end_pos as f32 - start_pos as f32) * scale) + start_pos as f32
    }

    /// `EnRu1_InitPositionFromFountainCue` (also `_InitPositionDivingIntoFountain`,
    /// `_InitPositionResurfacing`).
    fn init_position_from_fountain_cue(&mut self, play: &PlayState) {
        if let Some(cue) = Self::get_fountain_cue(play) {
            self.actor.shape_rot.y = cue.rot[1];
            self.actor.world_rot.y = cue.rot[1];
            self.actor.world_pos = cue_start(&cue);
        }
    }

    /// `EnRu1_SwimTowardLink`: along the cue.
    fn swim_toward_link(&mut self, play: &PlayState) {
        if let Some(cue) = Self::get_fountain_cue(play) {
            let t = Self::get_swim_lerp_factor(&cue, play);
            self.actor.world_pos.x = Self::lerp(t, cue.start_pos.x, cue.end_pos.x);
            self.actor.world_pos.y = Self::lerp(t, cue.start_pos.y, cue.end_pos.y);
            self.actor.world_pos.z = Self::lerp(t, cue.start_pos.z, cue.end_pos.z);
        }
    }

    /// `EnRu1_SwimAwayFromLink`: from where she trod to the cue's end. The `s32` lerp takes the
    /// tread position truncated.
    fn swim_away_from_link(&mut self, play: &PlayState) {
        if let Some(cue) = Self::get_fountain_cue(play) {
            let t = Self::get_swim_lerp_factor(&cue, play);
            let s = self.tread_start_pos;
            self.actor.world_pos.x = Self::lerp(t, s.x as i32, cue.end_pos.x);
            self.actor.world_pos.y = Self::lerp(t, s.y as i32, cue.end_pos.y);
            self.actor.world_pos.z = Self::lerp(t, s.z as i32, cue.end_pos.z);
        }
    }

    /// `EnRu1_PlayDivingEntrySfx`: once.
    fn play_diving_entry_sfx(&mut self, play: &mut PlayState) {
        if self.unk_298 == 0 {
            sfx(play, NA_SE_EV_DIVE_INTO_WATER);
            self.unk_298 = 1;
        }
    }

    /// `EnRu1_PlayResurfacingSfx`.
    fn play_resurfacing_sfx(&self, play: &mut PlayState) {
        if self.skel.on_frame(5.0) {
            sfx(play, NA_SE_PL_FACE_UP);
        }
    }

    /// `EnRu1_PlaySwimStrokeSfx`.
    fn play_swim_stroke_sfx(&self, play: &mut PlayState) {
        if self.skel.on_frame(4.0) || self.skel.on_frame(13.0) || self.skel.on_frame(22.0) || self.skel.on_frame(31.0) {
            sfx(play, NA_SE_PL_SWIM);
        }
    }

    /// `EnRu1_PlaySubmergeSfx`.
    fn play_submerge_sfx(&self, play: &mut PlayState) {
        if self.skel.on_frame(8.0) {
            sfx(play, NA_SE_PL_SUBMERGE);
        }
    }

    /// `EnRu1_PlayLinkFallingSfx`: Link's splash into the fountain, frame 205.
    fn play_link_falling_sfx(play: &mut PlayState) {
        if play.cs_ctx.frames == 205 {
            sfx_at_player(play, NA_SE_EV_DIVE_INTO_WATER);
        }
    }

    /// `EnRu1_PlayGiggleSfx`: frame 130.
    fn play_giggle_sfx(play: &mut PlayState) {
        if play.cs_ctx.frames == 130 {
            sfx(play, NA_SE_VO_RT_LAUGH_0);
        }
    }

    /// `EnRu1_StartBackSwimming`.
    fn start_back_swimming(&mut self, is_swimming: bool) {
        if is_swimming {
            self.change(self.anims.swim_on_back.clone(), ANIMMODE_LOOP, -8.0);
        }
    }

    /// `EnRu1_EnterFountainWater`: past cue 2, the dive.
    fn enter_fountain_water(&mut self, play: &mut PlayState) {
        if Self::check_fountain_cue_not_matching_id(play, 2) {
            self.action = ENRU1_ACTION_FOUNTAIN_DIVING;
            self.draw_config = ENRU1_DRAW_NOTHING;
            self.init_position_from_fountain_cue(play);
            self.update_bg_check_info(play);
            self.spawn_splash(play);
            self.spawn_three_ripples(play);
        }
    }

    /// `EnRu1_Resurface`: past cue 3.
    fn resurface(&mut self, play: &PlayState) {
        if Self::check_fountain_cue_not_matching_id(play, 3) {
            let a = self.anims.resurface.clone();
            let frame_count = a.last_frame();
            self.init_position_from_fountain_cue(play);
            self.move_in_water();
            self.skel.change(a, 1.0, 0.0, frame_count, ANIMMODE_ONCE, 0.0);
            self.action = ENRU1_ACTION_FOUNTAIN_RESURFACING;
            self.draw_config = ENRU1_DRAW_OPA;
        }
    }

    /// `EnRu1_ResurfaceProgress`.
    fn resurface_progress(&mut self, is_resurfaced: bool) {
        if is_resurfaced {
            self.stop_moving_in_water();
            self.change(self.anims.tread_water.clone(), ANIMMODE_LOOP, -8.0);
            self.action = ENRU1_ACTION_FOUNTAIN_TREADING_WATER;
        } else {
            // EnRu1_StartSwimmingTowardLink.
            self.start_moving_in_water();
        }
    }

    /// `EnRu1_StopTreading`: cue 6.
    fn stop_treading(&mut self, play: &PlayState) {
        if Self::check_fountain_cue_matching_id(play, 6) {
            self.move_in_water();
            self.change(self.anims.transition_to_swim_on_back.clone(), ANIMMODE_ONCE, -8.0);
            self.action = ENRU1_ACTION_FOUNTAIN_STARTING_SWIM_BACK;
        }
    }

    /// `EnRu1_TransitionToBackSwimming`.
    fn transition_to_back_swimming(&mut self, play: &PlayState) {
        if Self::check_fountain_cue_not_matching_id(play, 6) {
            self.stop_moving_in_water();
            self.action = ENRU1_ACTION_FOUNTAIN_SWIMMING_BACK;
            self.tread_start_pos = self.actor.world_pos;
        } else {
            // EnRu1_StartSwimmingAwayFromLink.
            self.start_moving_in_water();
        }
    }

    /// `EnRu1_EndSwimBack`: two frames before the cue's end.
    fn end_swim_back(&mut self, play: &PlayState) {
        if let Some(cue) = Self::get_fountain_cue(play) {
            let cs_cur_frame = play.cs_ctx.frames as i32;
            let end_frame = cue.end_frame as i32;
            if cs_cur_frame >= end_frame - 2 {
                self.change(self.anims.transition_from_swim_on_back.clone(), ANIMMODE_ONCE, -8.0);
                self.action = ENRU1_ACTION_FOUNTAIN_FINISHING_SWIM_BACK;
            }
        }
    }

    /// `EnRu1_EndGivingSapphire`: cue 8, turned back: gone.
    fn end_giving_sapphire(&mut self, play: &PlayState, anim_finished: bool) {
        if Self::check_fountain_cue_matching_id(play, 8) && anim_finished {
            self.actor.kill();
        }
    }

    // Type 2: the holes room.

    /// `EnRu1_InitInJabuJabuHolesRoom`.
    fn init_in_jabu_jabu_holes_room(&mut self, play: &PlayState) {
        if !play.save.get_inf_table(INFTABLE_RUTO_MET_FIRST_TIME) {
            self.animation_change(self.anims.wait2.clone(), ANIMMODE_LOOP, 0.0, false);
            self.action = ENRU1_ACTION_MEETING_RANGE_CHECK;
            self.set_mouth(ENRU1_MOUTH_FROWNING);
        } else if play.save.get_inf_table(INFTABLE_RUTO_BROUGHT_BACK_TO_HOLES_ROOM) && !play.save.get_inf_table(INFTABLE_RUTO_PLACED_ON_SWITCH) && !play.save.get_inf_table(INFTABLE_RUTO_HAS_SAPPHIRE) {
            if !self.is_assisting_link(play) {
                self.speakable_pre_spawn_init(true);
            } else {
                self.actor.kill();
            }
        } else {
            self.actor.kill();
        }
    }

    /// The speakable inits' common part: `gRutoChildWait2Anim`, `ENRU1_ACTION_SPEAK_JABU_PRE_SPAWN`,
    /// following rooms (`room` -1), the room she was placed in kept thrice.
    fn speakable_pre_spawn_init(&mut self, hidden: bool) {
        self.animation_change(self.anims.wait2.clone(), ANIMMODE_LOOP, 0.0, false);
        let actor_room = self.actor.room;
        self.action = ENRU1_ACTION_SPEAK_JABU_PRE_SPAWN;
        self.actor.room = -1;
        if hidden {
            self.draw_config = ENRU1_DRAW_NOTHING;
        }
        self.room_num1 = actor_room;
        self.room_num3 = actor_room;
        self.room_num2 = actor_room;
    }

    /// `EnRu1_AccelerateAway`: up to 2.7 over eight frames.
    fn accelerate_away(&mut self) {
        let walking_frame = self.walking_frame;
        if walking_frame < 8.0 {
            self.actor.speed_xz = (((KREG as f32 * 0.01) + 2.7) / 8.0) * walking_frame;
        } else {
            self.actor.speed_xz = (KREG as f32 * 0.01) + 2.7;
        }
        self.actor.velocity.y = -1.0;
        self.actor.move_forward();
    }

    /// `EnRu1_MoveForwardConstant`.
    fn move_forward_constant(&mut self) {
        self.actor.velocity.y = -1.0;
        self.actor.move_forward();
    }

    /// `EnRu1_AccelerateDownHole`: down to 13 a frame over eight frames, her speed easing out.
    fn accelerate_down_hole(&mut self) {
        if self.walking_frame < 8.0 {
            self.walking_frame += 1.0;
            self.actor.speed_xz *= (8.0 - self.walking_frame) / 8.0;
            self.actor.velocity.y = -self.walking_frame * (((KREG as f32 * 0.01) + 13.0) / 8.0);
        } else {
            self.actor.speed_xz = 0.0;
            self.actor.velocity.y = -((KREG as f32 * 0.01) + 13.0);
        }
        self.actor.move_forward();
    }

    /// `EnRu1_IsPlayerInRangeForFirstEncounter`: within 265 along z, not below her.
    fn is_player_in_range_for_first_encounter(&self, play: &PlayState) -> bool {
        let p = player_pos(play);
        p.z - self.actor.world_pos.z <= 265.0 && p.y >= self.actor.world_pos.y
    }

    /// `EnRu1_PlayStepSfx`: frames 2 and 7, on Jabu's surface, until she falls.
    fn play_step_sfx(&self, play: &mut PlayState) {
        if self.is_falling == 0 && (self.skel.on_frame(2.0) || self.skel.on_frame(7.0)) {
            sfx(play, NA_SE_PL_WALK_GROUND + SURFACE_SFX_OFFSET_JABU);
        }
    }

    /// `EnRu1_PlayFallingSfx`.
    fn play_falling_sfx(play: &mut PlayState) {
        sfx(play, NA_SE_EV_FALL_DOWN_DIRT);
        sfx(play, NA_SE_VO_RT_FALL);
    }

    /// `EnRu1_RespondToFalling`: cue 4, her arms up.
    fn respond_to_falling(&mut self, play: &mut PlayState) {
        if Self::check_cue_matching_id(play, 4, 3) && self.is_falling == 0 {
            self.change(self.anims.bring_arms_up.clone(), ANIMMODE_ONCE, -8.0);
            self.is_falling = 1;
            Self::play_falling_sfx(play);
        }
    }

    /// `EnRu1_CheckStartFirstEncounter`: Link in range on the ground, not climbing, hanging or in a
    /// cutscene: `gRutoFirstMeetingCs`, Link stopped.
    fn check_start_first_encounter(&mut self, play: &mut PlayState) {
        let Some(p) = player(play) else { return };
        let (state1, on_ground) = (p.state1, p.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0);
        if self.is_player_in_range_for_first_encounter(play) && !play.play_in_cs_mode() && state1 & (STATE1_13 | STATE1_14 | STATE1_21) == 0 && on_ground {
            play.cs_ctx.segment = play.cutscene_script("gRutoFirstMeetingCs");
            play.save.cutscene_trigger = 1;
            if let Some(pi) = play.player.and_then(|h| play.actors.get_mut(h)).and_then(|p| p.as_player_mut()) {
                pi.set_player_speed(0.0);
            }
            self.action = ENRU1_ACTION_MEETING_INIT_POSITION;
        }
    }

    /// `EnRu1_SetupPositionForFirstEncounter`: past cue 1, at the cue facing Link, shown.
    fn setup_position_for_first_encounter(&mut self, play: &PlayState) {
        if Self::check_cue_not_matching_id(play, 1, 3)
            && let Some(cue) = play.cs_ctx.npc_actions[3]
        {
            self.actor.world_pos = cue_start(&cue);
            self.actor.shape_rot.y = cue.rot[1];
            self.actor.world_rot.y = cue.rot[1];
            self.action = ENRU1_ACTION_MEETING_FACING_LINK;
            self.draw_config = ENRU1_DRAW_OPA;
        }
    }

    /// `EnRu1_CheckTurnAround`: cue 3.
    fn check_turn_around(&mut self, play: &PlayState) {
        if Self::check_cue_matching_id(play, 3, 3) {
            self.change(self.anims.turn_around.clone(), ANIMMODE_ONCE, -8.0);
            self.action = ENRU1_ACTION_MEETING_TURNING_AROUND;
        }
    }

    /// `EnRu1_StartWalkingAway`: turned round.
    fn start_walking_away(&mut self, done_turning: bool) {
        if done_turning {
            self.change(self.anims.walk.clone(), ANIMMODE_LOOP, -8.0);
            self.actor.world_rot.y = self.actor.world_rot.y.wrapping_add(0x8000u16 as i16);
            self.action = ENRU1_ACTION_MEETING_WALKING_AWAY_ACCEL;
            self.walking_frame = 0.0;
        }
    }

    /// `EnRu1_AdvanceWalkingAway`.
    fn advance_walking_away(&mut self) {
        self.walking_frame += 1.0;
        if self.walking_frame >= 8.0 {
            self.action = ENRU1_ACTION_MEETING_WALKING_AWAY_CONSTANT;
            self.walking_frame = 0.0;
            self.actor.velocity.y = -1.0;
        }
    }

    /// `EnRu1_CheckStartFalling`: off the floor.
    fn check_start_falling(&mut self) {
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND == 0 {
            self.action = ENRU1_ACTION_MEETING_FALLING_DOWN_HOLE;
            self.walking_frame = 0.0;
            self.actor.velocity.y = 0.0;
        }
    }

    /// `EnRu1_FinishFirstEncounter`: cue 5.
    fn finish_first_encounter(&mut self, play: &mut PlayState) {
        if Self::check_cue_matching_id(play, 5, 3) {
            play.save.set_inf_table(INFTABLE_RUTO_MET_FIRST_TIME);
            self.action = ENRU1_ACTION_MEETING_END;
        }
    }

    // Type 0: the boss room.

    /// `EnRu1_SpawnBlueWarp`: `WARP_BLUE_RUTO` 12 towards Link, as her child.
    fn spawn_blue_warp(&mut self, play: &mut PlayState) {
        let yaw = self.actor.yaw_towards_player;
        let pos = self.actor.world_pos;
        let spawn = Vec3::new(((KREG as f32 + 12.0) * sin_s(yaw)) + pos.x, pos.y, ((KREG as f32 + 12.0) * cos_s(yaw)) + pos.z);
        self.blue_warp = match play.actor_spawn_as_child(&mut self.actor, ACTOR_DOOR_WARP1, spawn, [0, yaw, 0], WARP_BLUE_RUTO) {
            Ok(h) => Some(h),
            Err(e) => {
                log::warn!("En_Ru1: her blue warp wasn't spawned: {e:?}");
                None
            }
        };
    }

    /// `EnRu1_InitInBossRoom`.
    fn init_in_boss_room(&mut self) {
        self.animation_change(self.anims.wait_hands_on_hips.clone(), ANIMMODE_LOOP, 0.0, false);
        self.action = ENRU1_ACTION_PRE_SPAWN_BOSS_ROOM;
        self.actor.shape_y_offset = -10000.0;
        self.set_eyes(ENRU1_EYES_BLUSH);
        self.set_mouth(ENRU1_MOUTH_OPEN);
    }

    /// `EnRu1_Rise`.
    fn rise(&mut self) {
        self.actor.shape_y_offset += 250.0 / 3.0;
    }

    /// `EnRu1_RiseWithLink`: at her distance along Link's facing.
    fn rise_with_link(&mut self, play: &PlayState) {
        let Some(p) = play.player.and_then(|h| play.actors.actor(h)) else { return };
        let (player_pos, shape_rot_y) = (p.world_pos, p.shape_rot.y);
        let d = self.xz_dist_to_player_in_blue_warp;
        self.actor.world_pos.x = (sin_s(shape_rot_y) * d) + player_pos.x;
        self.actor.world_pos.y = player_pos.y;
        self.actor.world_pos.z = (cos_s(shape_rot_y) * d) + player_pos.z;
    }

    /// `EnRu1_SetPlayerMarkInBlueWarp`: Link's `unk_450` 30 in front of her.
    fn set_player_mark_in_blue_warp(&self, play: &mut PlayState) {
        let pos = self.actor.world_pos;
        let rot_y = self.actor.shape_rot.y;
        if let Some(p) = play.player.and_then(|h| play.actors.downcast_mut::<Player>(h)) {
            p.unk_450.x = ((KREG as f32 + 30.0) * sin_s(rot_y)) + pos.x;
            p.unk_450.z = ((KREG as f32 + 30.0) * cos_s(rot_y)) + pos.z;
        }
    }

    /// `EnRu1_IsLinkInBlueWarp`: 3 frames on and Link still, he's put on his mark and turned to
    /// face her (`Math_SmoothStepToS`); true once he faces her.
    fn is_link_in_blue_warp(&mut self, play: &mut PlayState) -> bool {
        self.walking_frame += 1.0;
        let pos = self.actor.world_pos;
        let Some(p) = play.player.and_then(|h| play.actors.downcast_mut::<Player>(h)) else { return false };
        if p.actor.speed_xz == 0.0 && self.walking_frame >= 3.0 {
            p.actor.world_pos = p.unk_450;
            let dx = pos.x - p.actor.world_pos.x;
            let dz = pos.z - p.actor.world_pos.z;
            let target_rot_y = oot_game::path::rad_to_binang_trunc(oot_game::camera::f_atan2f(dx, dz));
            if p.actor.shape_rot.y != target_rot_y {
                smooth_step_to_s(&mut p.actor.shape_rot.y, target_rot_y, 0x14, 0x1838, 0x64);
                p.actor.world_rot.y = p.actor.shape_rot.y;
            } else {
                return true;
            }
        }
        false
    }

    fn blue_warp<'a>(&self, play: &'a PlayState) -> Option<&'a DoorWarp1> {
        self.blue_warp.and_then(|h| play.actors.downcast::<DoorWarp1>(h))
    }

    /// `EnRu1_CheckBlueWarpState`.
    fn check_blue_warp_state(&self, play: &PlayState, state: i32) -> bool {
        self.blue_warp(play).is_some_and(|w| w.ruto_warp_state == state)
    }

    /// `EnRu1_SetBlueWarpState`.
    fn set_blue_warp_state(&self, play: &mut PlayState, state: i32) {
        if let Some(w) = self.blue_warp.and_then(|h| play.actors.downcast_mut::<DoorWarp1>(h)) {
            w.ruto_warp_state = state;
        }
    }

    /// `EnRu1_SetupRiseThroughBlueWarp`: turned to Link, shown, her warp spawned.
    fn setup_rise_through_blue_warp(&mut self, play: &mut PlayState) {
        self.action = ENRU1_ACTION_RISE_THROUGH_BLUE_WARP;
        self.draw_config = ENRU1_DRAW_OPA;
        self.actor.world_rot.y = self.actor.yaw_towards_player;
        self.actor.shape_rot.y = self.actor.yaw_towards_player;
        self.spawn_blue_warp(play);
    }

    /// `EnRu1_EndRise`.
    fn end_rise(&mut self, play: &mut PlayState) {
        if self.actor.shape_y_offset >= 0.0 {
            self.action = ENRU1_ACTION_WAIT_INSIDE_BLUE_WARP;
            self.actor.shape_y_offset = 0.0;
            self.set_blue_warp_state(play, WARP_BLUE_RUTO_STATE_READY);
        }
    }

    /// `EnRu1_CheckLinkEnteredBlueWarp`.
    fn check_link_entered_blue_warp(&mut self, play: &mut PlayState) {
        if self.check_blue_warp_state(play, WARP_BLUE_RUTO_STATE_ENTERED) {
            self.action = ENRU1_ACTION_LINK_WALKS_TO_POINT_IN_BLUE_WARP;
            self.walking_frame = 0.0;
            self.set_player_mark_in_blue_warp(play);
        }
    }

    /// `EnRu1_StartCrossingArmsAndLegs`.
    fn start_crossing_arms_and_legs(&mut self, play: &mut PlayState, should_start: bool) {
        if should_start {
            self.change(self.anims.transition_hands_on_hip_to_cross_arms_and_legs.clone(), ANIMMODE_ONCE, -8.0);
            self.action = ENRU1_ACTION_WHAT_TOOK_YOU_SO_LONG;
            self.set_blue_warp_state(play, WARP_BLUE_RUTO_STATE_3);
        }
    }

    /// `EnRu1_AdvanceAngryAnimation`.
    fn advance_angry_animation(&mut self, play: &PlayState, is_talking: bool) {
        if self.check_blue_warp_state(play, WARP_BLUE_RUTO_STATE_TALKING) {
            if is_talking {
                self.change(self.anims.wait_sitting.clone(), ANIMMODE_LOOP, -8.0);
            }
        } else if self.check_blue_warp_state(play, WARP_BLUE_RUTO_STATE_WARPING) {
            self.change(self.anims.wait_in_blue_warp.clone(), ANIMMODE_ONCE, -8.0);
            self.action = ENRU1_ACTION_WARPING_OUT;
            self.xz_dist_to_player_in_blue_warp = self.actor.xz_dist_to_player;
        }
    }

    // Types 3 and 6 (and 2, brought back): the speakable Ruto.

    /// `EnRu1_InitInJabuJabuBasement`.
    fn init_in_jabu_jabu_basement(&mut self, play: &PlayState) {
        let s = &play.save;
        if s.get_inf_table(INFTABLE_RUTO_MET_FIRST_TIME) && !s.get_inf_table(INFTABLE_RUTO_HAS_SAPPHIRE) && !s.get_inf_table(INFTABLE_RUTO_PLACED_ON_SWITCH) && !s.get_inf_table(INFTABLE_RUTO_BROUGHT_BACK_TO_HOLES_ROOM) {
            if !self.is_assisting_link(play) {
                // (Not hidden: drawConfig stays as it was, ENRU1_DRAW_NOTHING, from the init.)
                self.speakable_pre_spawn_init(false);
            } else {
                self.actor.kill();
            }
        } else {
            self.actor.kill();
        }
    }

    /// `EnRu1_PlayHittingSurfaceSfx`.
    fn play_hitting_surface_sfx(play: &mut PlayState) {
        sfx(play, NA_SE_EV_LAND_DIRT);
    }

    /// `EnRu1_PlayLiftingSfx`: Link's pull and her cry.
    fn play_lifting_sfx(play: &mut PlayState) {
        sfx_at_player(play, NA_SE_PL_PULL_UP_RUTO);
        sfx(play, NA_SE_VO_RT_LIFT);
    }

    /// `func_80AED57C`: thrown, her cry.
    fn func_80aed57c(&self, play: &mut PlayState) {
        if self.actor.speed_xz != 0.0 {
            sfx(play, NA_SE_VO_RT_THROW);
        }
    }

    /// `EnRu1_PlayBouncingOffWallSfx`.
    fn play_bouncing_off_wall_sfx(play: &mut PlayState) {
        sfx(play, NA_SE_VO_RT_CRASH);
    }

    /// `func_80AED5DC`.
    fn func_80aed5dc(play: &mut PlayState) {
        sfx(play, NA_SE_VO_RT_UNBALLANCE);
    }

    /// `func_80AED600`.
    fn func_80aed600(play: &mut PlayState) {
        sfx(play, NA_SE_VO_RT_DISCOVER);
    }

    /// `func_80AED624`: gone in a room she wasn't let go in; over 50 deep in a room she wasn't
    /// placed in, fading out.
    fn func_80aed624(&mut self, play: &PlayState) -> bool {
        let cur_room_num = Self::cur_room(play);
        if self.room_num2 != cur_room_num {
            self.actor.kill();
            return false;
        } else if (self.room_num1 != cur_room_num || self.room_num2 != cur_room_num)
            // roomNum2 is always curRoomNum here.
            && self.actor.y_dist_to_water > KREG as f32 + 50.0
            && self.action != ENRU1_ACTION_SITTING_DISAPPEARING
        {
            self.action = ENRU1_ACTION_SITTING_DISAPPEARING;
            self.draw_config = ENRU1_DRAW_XLU;
            self.alpha = 255;
            self.sink_timer = 0.0;
        }
        true
    }

    /// `func_80AED6DC`: let go in this room.
    fn func_80aed6dc(&mut self, play: &PlayState) {
        self.room_num2 = Self::cur_room(play);
        self.unk_288 = 0.0;
    }

    /// `EnRu1_CheckIfBackInHolesRoom`: carried into room 2.
    fn check_if_back_in_holes_room(play: &mut PlayState) {
        if !play.save.get_inf_table(INFTABLE_RUTO_BROUGHT_BACK_TO_HOLES_ROOM) && Self::cur_room(play) == 2 {
            play.save.set_inf_table(INFTABLE_RUTO_BROUGHT_BACK_TO_HOLES_ROOM);
        }
    }

    /// `EnRu1_FadeWhileSinking`: out over 20 frames, then gone.
    fn fade_while_sinking(&mut self, play: &PlayState) {
        if self.func_80aed624(play) {
            self.sink_timer += 1.0;
            if self.sink_timer < 20.0 {
                let alpha = (((20.0 - self.sink_timer) * 255.0) / 20.0) as u32;
                self.alpha = alpha as i32;
                self.actor.shape_shadow_alpha = alpha as u8;
            } else {
                self.actor.kill();
            }
        }
    }

    /// `EnRu1_TurnUpperBodyForward`.
    fn turn_upper_body_forward(&mut self) {
        let t = &mut self.interact_info;
        smooth_step_to_s(&mut t.head[0], 0, 0x14, 0x1838, 0x64);
        smooth_step_to_s(&mut t.head[1], 0, 0x14, 0x1838, 0x64);
        smooth_step_to_s(&mut t.torso[0], 0, 0x14, 0x1838, 0x64);
        smooth_step_to_s(&mut t.torso[1], 0, 0x14, 0x1838, 0x64);
    }

    /// `EnRu1_UpdateHeadRotation`: carried, she looks about (ahead, right or left, a new choice
    /// each 10 to 35 frames).
    fn update_head_rotation(&mut self, play: &mut PlayState) {
        if decr(&mut self.head_rot_timer) == 0 {
            self.head_rot_timer = play.rand.s16_offset(0xA, 0x19);
            let head_rot_offset = self.head_rot_timer % 5;
            self.head_rot_direction = match head_rot_offset {
                0 => 1,
                1 => 2,
                _ => 0,
            };
            self.head_turn_speed = 0;
        }
        let head_rot_y = self.interact_info.head[1];
        let target: i16 = match self.head_rot_direction {
            0 => 0,
            1 => -0x2AAA,
            _ => 0x2AAA,
        };
        smooth_step_to_s(&mut self.head_turn_speed, (target as i32 - head_rot_y as i32) as i16, 1, 0x190, 0x190);
        let step = (self.head_turn_speed as i32).abs() as i16;
        smooth_step_to_s(&mut self.interact_info.head[1], target, 3, step, 0x64);
    }

    /// `DynaPoly_GetActor(&play->colCtx, floorBgId)`: the bg actor under her, its id.
    fn floor_actor_id(&self, play: &PlayState) -> Option<i16> {
        dyna_poly_get_actor(&play.actors, &play.col, self.actor.floor_bg_id).and_then(|h| play.actors.actor(h)).map(|a| a.id)
    }

    /// `EnRu1_ResetBgCheckFlags`: on the scene's floor or a chest, her floor, wall and ceiling
    /// flags dropped.
    fn reset_bg_check_flags(&mut self, play: &PlayState) {
        let id = self.floor_actor_id(play);
        if id.is_none() || id == Some(ACTOR_EN_BOX) {
            self.actor.bg_check_flags &= !(BGCHECKFLAG_GROUND | BGCHECKFLAG_WALL | BGCHECKFLAG_CEILING);
        }
    }

    /// `EnRu1_UpdateBouncing`: off floors, ceilings and walls.
    fn update_bouncing(&mut self, play: &mut PlayState) {
        let min_speed = (KREG as f32 * 0.01) + 3.0;
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
            let dyna = self.floor_actor_id(play);
            if self.actor.velocity.y <= 0.0 {
                match dyna {
                    Some(id) => {
                        if id != ACTOR_EN_BOX {
                            self.actor.speed_xz = 0.0;
                        }
                    }
                    None => {
                        if self.actor.speed_xz >= min_speed {
                            self.actor.speed_xz *= (KREG as f32 * 0.01) + 0.8;
                        } else {
                            self.actor.speed_xz = 0.0;
                        }
                    }
                }
                match dyna {
                    Some(id) => {
                        if id != ACTOR_EN_BOX {
                            self.actor.velocity.y = 0.0;
                            self.actor.min_velocity_y = 0.0;
                            self.actor.gravity = 0.0;
                        } else {
                            self.actor.velocity.y *= -1.0;
                        }
                    }
                    None => {
                        self.actor.velocity.y *= -((KREG as f32 * 0.01) + 0.6);
                        if self.actor.velocity.y <= -self.actor.gravity * ((KREG as f32 * 0.01) + 0.6) {
                            self.actor.velocity.y = 0.0;
                            self.actor.min_velocity_y = 0.0;
                            self.actor.gravity = 0.0;
                        }
                    }
                }
                Self::play_hitting_surface_sfx(play);
            }
        }
        if self.actor.bg_check_flags & BGCHECKFLAG_CEILING != 0 {
            if self.actor.speed_xz >= min_speed {
                self.actor.speed_xz *= (KREG as f32 * 0.01) + 0.8;
            } else {
                self.actor.speed_xz = 0.0;
            }
            if self.actor.velocity.y >= 0.0 {
                self.actor.velocity.y *= -((KREG as f32 * 0.01) + 0.6);
                Self::play_hitting_surface_sfx(play);
            }
        }
        if self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 && self.actor.speed_xz != 0.0 {
            let rot_y = self.actor.world_rot.y as i32;
            let wall_yaw = self.actor.wall_yaw as i32;
            let refl_y = (wall_yaw * 2) - rot_y;
            let exit_y = refl_y + 0x8000;
            let normal_angle: i32 = if ((refl_y - wall_yaw) + 0x8000) as i16 >= 0 { ((exit_y - wall_yaw) as i16) as i32 } else { -(((exit_y - wall_yaw) as i16) as i32) };
            if normal_angle < 0x4001 {
                if self.actor.speed_xz >= min_speed {
                    self.actor.speed_xz *= (KREG as f32 * 0.01) + 0.6;
                } else {
                    self.actor.speed_xz = 0.0;
                }
                self.actor.world_rot.y = exit_y as i16;
                Self::play_hitting_surface_sfx(play);
                Self::play_bouncing_off_wall_sfx(play);
            }
        }
    }

    /// `EnRu1_UpdateSpeedXZ`: a chest speeds her, else 0.98 a frame to under 3.
    fn update_speed_xz(&mut self, play: &PlayState) {
        if self.floor_actor_id(play) == Some(ACTOR_EN_BOX) {
            if self.actor.speed_xz != 0.0 {
                self.actor.speed_xz *= 1.1;
            } else {
                self.actor.speed_xz = 1.0;
            }
        }
        if self.actor.speed_xz >= (KREG as f32 * 0.01) + 3.0 {
            self.actor.speed_xz *= (KREG as f32 * 0.01) + 0.98;
        } else {
            self.actor.speed_xz = 0.0;
        }
    }

    /// `EnRu1_UpdatePosition`.
    fn update_position(&mut self, play: &mut PlayState) {
        self.update_bouncing(play);
        self.update_speed_xz(play);
        self.actor.move_forward();
    }

    /// `EnRu1_StopMoving`.
    fn stop_moving(&mut self) {
        self.actor.velocity = Vec3::ZERO;
        self.actor.speed_xz = 0.0;
        self.actor.gravity = 0.0;
        self.actor.min_velocity_y = 0.0;
    }

    /// `EnRu1_UpdateWaterState`: slowed by the water, then bobbing, then sinking.
    fn update_water_state(&mut self) {
        if self.water_state == ENRU1_WATER_OUTSIDE {
            if self.actor.min_velocity_y == 0.0 && self.actor.speed_xz == 0.0 {
                // When Ruto's velocity has been slowed enough by the water, stop her motion.
                self.water_state = ENRU1_WATER_IMMERSED;
                self.stop_moving();
                self.bob_phase = 0;
                self.bob_depth = (self.actor.y_dist_to_water - 10.0) * 0.5;
                self.sinking_start_pos_y = self.actor.world_pos.y + self.bob_depth;
            } else {
                // Ruto is touching the water but still in motion, e.g. from being thrown.
                self.actor.gravity = 0.0;
                self.actor.min_velocity_y *= 0.2;
                self.actor.velocity.y *= 0.2;
                if self.actor.min_velocity_y >= -0.1 {
                    self.actor.min_velocity_y = 0.0;
                    self.actor.velocity.y = 0.0;
                }
                self.actor.speed_xz *= 0.5;
                if self.actor.speed_xz <= 0.1 {
                    self.actor.speed_xz = 0.0;
                }
                self.actor.velocity.x = sin_s(self.actor.world_rot.y) * self.actor.speed_xz;
                self.actor.velocity.z = cos_s(self.actor.world_rot.y) * self.actor.speed_xz;
                self.actor.update_pos();
            }
        } else if self.water_state == ENRU1_WATER_IMMERSED {
            if self.bob_depth <= 1.0 {
                self.stop_moving();
                self.water_state = ENRU1_WATER_BOBBING;
                self.is_sinking = 0.0;
            } else {
                let bob_magnitude = self.bob_depth;
                let start_y = self.sinking_start_pos_y;
                let delta_y = cos_s(self.bob_phase) * -bob_magnitude;
                self.actor.world_pos.y = delta_y + start_y;
                self.bob_phase = self.bob_phase.wrapping_add(0x3E8);
                self.bob_depth *= 0.95;
            }
        } else {
            self.is_sinking += 1.0;
            if self.is_sinking > 0.0 {
                self.water_state = ENRU1_WATER_SINKING;
            }
        }
    }

    /// `EnRu1_TalkOfferAccepted`: else her text by the flags offered.
    fn talk_offer_accepted(&mut self, play: &mut PlayState) -> bool {
        if !process_talk_request(&mut self.actor) {
            self.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY;
            self.actor.text_id = Self::speakable_text_id(play);
            let a = self.actor.clone();
            offer_talk_default(play, &a);
            return false;
        }
        true
    }

    /// `EnRu1_TalkOfferAccepted`'s text: 0x404E carried before, 0x404D explained, else 0x404C.
    pub fn speakable_text_id(play: &PlayState) -> u16 {
        if play.save.get_inf_table(INFTABLE_RUTO_LET_LINK_CARRY) {
            0x404E
        } else if play.save.get_inf_table(INFTABLE_RUTO_EXPLAINED_WHY_IN_JABU) {
            0x404D
        } else {
            0x404C
        }
    }

    /// `EnRu1_CheckForDoorSwitch`: on a blue switch with flag 0x38 the flag's set, else cleared.
    fn check_for_door_switch(&self, play: &mut PlayState) {
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 && self.actor.floor_bg_id != BGCHECK_SCENE {
            let a = dyna_poly_get_actor(&play.actors, &play.col, self.actor.floor_bg_id).and_then(|h| play.actors.actor(h));
            // schema: another actor's params read whole (Bg_Bdan_Switch's switch flag, PARAMS_GET_U(params, 8, 6))
            if a.is_some_and(|a| a.id == ACTOR_BG_BDAN_SWITCH && (a.params >> 8) & 0x3F == 0x38) {
                play.save.set_inf_table(INFTABLE_RUTO_PLACED_ON_SWITCH);
                return;
            }
        }
        play.save.clear_inf_table(INFTABLE_RUTO_PLACED_ON_SWITCH);
    }

    /// `EnRu1_IsOnSapphirePlatform`: landed on Big Octo's platform with no cutscene or box:
    /// `gRutoObtainingSapphireCs`, hidden.
    fn is_on_sapphire_platform(&mut self, play: &mut PlayState) -> bool {
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 && self.actor.floor_bg_id != BGCHECK_SCENE {
            let h = dyna_poly_get_actor(&play.actors, &play.col, self.actor.floor_bg_id);
            // schema: another actor's params read whole (Bg_Bdan_Objects', cut to its type at its init)
            let platform = h.filter(|&h| play.actors.actor(h).is_some_and(|a| a.id == ACTOR_BG_BDAN_OBJECTS && a.params == 0));
            if let Some(h) = platform
                && !play.player_in_cs_mode()
                && play.msg_ctx.msg_length == 0
            {
                self.stop_moving();
                play.cs_ctx.segment = play.cutscene_script("gRutoObtainingSapphireCs");
                play.save.cutscene_trigger = 1;
                self.action = ENRU1_ACTION_THROWN_ONTO_SAPPHIRE_PLATFORM;
                self.draw_config = ENRU1_DRAW_NOTHING;
                self.big_octo_platform = Some(h);
                self.actor.shape_shadow_alpha = 0;
                return true;
            }
        }
        false
    }

    /// `EnRu1_UpdateSittingAction`: lifted, carried; off the floor, falling.
    fn update_sitting_action(&mut self, play: &mut PlayState) {
        if self.actor.parent.is_some() {
            // Ruto has been picked up.
            self.room_num3 = Self::cur_room(play);
            self.action = ENRU1_ACTION_SITTING_CARRIED;
            Self::play_lifting_sfx(play);
        } else if !self.is_on_sapphire_platform(play) && self.actor.bg_check_flags & BGCHECKFLAG_GROUND == 0 {
            self.actor.min_velocity_y = -((KREG as f32 * 0.01) + 6.8);
            self.actor.gravity = -((KREG as f32 * 0.01) + 1.3);
            self.action = ENRU1_ACTION_SITTING_RELEASED;
        }
    }

    /// `EnRu1_CheckLanding`: stopped on the floor, sitting; in water, entering it.
    fn check_landing(&mut self, play: &mut PlayState) {
        if !self.is_on_sapphire_platform(play) {
            if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 && self.actor.speed_xz == 0.0 && self.actor.min_velocity_y == 0.0 {
                self.stop_moving();
                let a = self.actor.clone();
                oot_game::get_item::offer_pick_up(play, &a);
                self.action = ENRU1_ACTION_SITTING_IDLE;
                self.disable_sitting_oc();
                return;
            }
            if self.actor.y_dist_to_water > 0.0 {
                self.action = ENRU1_ACTION_SITTING_ENTERING_WATER;
                self.water_state = ENRU1_WATER_OUTSIDE;
            }
        }
    }

    /// `EnRu1_EndSeeingSapphireAnimation`: the script over, sitting and carried again.
    fn end_seeing_sapphire_animation(&mut self, play: &mut PlayState) {
        let cur_room_num = Self::cur_room(play);
        if Self::is_cs_state_idle(play) {
            self.change(self.anims.sitting.clone(), ANIMMODE_LOOP, -8.0);
            play.save.set_inf_table(INFTABLE_RUTO_BROUGHT_TO_SAPPHIRE_ROOM);
            self.action = ENRU1_ACTION_SITTING_CARRIED;
        }
        self.room_num3 = cur_room_num;
    }

    /// `func_80AEE6D0`: carried where the sapphire is: `gRutoFoundSapphireCs`. `@bug (game)`:
    /// `gRutoChildSeesSapphireAnim` is played to `gRutoChildSquirmAnim`'s last frame.
    fn func_80aee6d0(&mut self, play: &mut PlayState) -> bool {
        let cur_room_num = Self::cur_room(play);
        if !play.save.get_inf_table(INFTABLE_RUTO_BROUGHT_TO_SAPPHIRE_ROOM) && Self::find_zora_jewel(play).is_some() {
            if !play.player_in_cs_mode() {
                let end = self.anims.squirm.last_frame();
                self.skel.change(self.anims.sees_sapphire.clone(), 1.0, 0.0, end, ANIMMODE_LOOP, -8.0);
                Self::func_80aed600(play);
                self.action = ENRU1_ACTION_SITTING_SEES_SAPPHIRE;
                self.walking_frame = 0.0;
                play.cs_ctx.segment = play.cutscene_script("gRutoFoundSapphireCs");
                play.save.cutscene_trigger = 1;
            }
            self.room_num3 = cur_room_num;
            return true;
        }
        self.room_num3 = cur_room_num;
        false
    }

    /// `EnRu1_UpdateCarriedBehavior`: let go, thrown or put down; else the sapphire, else her
    /// squirming while Link idles.
    fn update_carried_behavior(&mut self, play: &mut PlayState) {
        if self.actor.parent.is_none() {
            self.change(self.anims.sitting.clone(), ANIMMODE_LOOP, -8.0);
            self.func_80aed6dc(play);
            self.actor.speed_xz *= (KREG as f32 * 0.01) + 1.0;
            self.actor.velocity.y *= (KREG as f32 * 0.01) + 1.0;
            self.actor.min_velocity_y = -((KREG as f32 * 0.01) + 6.8);
            self.actor.gravity = -((KREG as f32 * 0.01) + 1.3);
            self.func_80aed57c(play);
            self.action = ENRU1_ACTION_SITTING_RELEASED;
            self.carry_idle_timer = 0.0;
        } else if self.func_80aee6d0(play) {
            self.carry_idle_timer = 0.0;
        } else {
            let fidget = player(play).is_some_and(|p| p.state2 & STATE2_28 != 0);
            if fidget {
                self.carry_idle_timer += 1.0;
                if self.action != ENRU1_ACTION_SITTING_LOSING_BALANCE {
                    if self.carry_idle_timer > 30.0 {
                        if play.rand.s16_offset(0, 3) == 0 {
                            self.change(self.anims.squirm.clone(), ANIMMODE_LOOP, -8.0);
                            Self::func_80aed5dc(play);
                            self.action = ENRU1_ACTION_SITTING_LOSING_BALANCE;
                        }
                        self.carry_idle_timer = 0.0;
                    }
                } else if self.carry_idle_timer > 50.0 {
                    self.change(self.anims.sitting.clone(), ANIMMODE_LOOP, -8.0);
                    self.action = ENRU1_ACTION_SITTING_CARRIED;
                    self.carry_idle_timer = 0.0;
                }
            } else {
                self.change(self.anims.sitting.clone(), ANIMMODE_LOOP, -8.0);
                self.carry_idle_timer = 0.0;
            }
        }
    }

    /// `EnRu1_CheckHitBottomUnderwater`: on the bottom, sitting.
    fn check_hit_bottom_underwater(&mut self, play: &mut PlayState) -> bool {
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
            self.stop_moving();
            let a = self.actor.clone();
            oot_game::get_item::offer_pick_up(play, &a);
            self.action = ENRU1_ACTION_SITTING_IDLE;
            self.disable_sitting_oc();
            return true;
        }
        false
    }

    /// `EnRu1_CheckSinkingState`.
    fn check_sinking_state(&mut self, play: &mut PlayState) {
        if !self.check_hit_bottom_underwater(play) && self.water_state == ENRU1_WATER_SINKING {
            self.action = ENRU1_ACTION_SITTING_SINKING;
            self.stop_moving();
            self.actor.gravity = -0.1;
            self.actor.min_velocity_y = -((KREG as f32 * 0.1) + 0.7);
        }
    }

    /// `EnRu1_OfferCarry`.
    fn offer_carry(&self, play: &mut PlayState) {
        let a = self.actor.clone();
        oot_game::get_item::offer_pick_up(play, &a);
    }

    /// `EnRu1_TurnUpperBodyTowardPlayer`: her head and torso on Link (preset 12).
    fn turn_upper_body_toward_player(&mut self, play: &mut PlayState) {
        self.interact_info.target = player_pos(play);
        self.interact_info.eye_height = KREG as f32 - 3.0;
        let preset = (KREG + 0xC) as usize;
        track_point(play, &mut self.actor, &mut self.interact_info, preset, NPC_TRACKING_HEAD_AND_TORSO);
    }

    /// `EnRu1_TurnFullBodyTowardPlayer`.
    fn turn_full_body_toward_player(&mut self, play: &mut PlayState) {
        self.interact_info.target = player_pos(play);
        self.interact_info.eye_height = KREG as f32 - 3.0;
        let preset = (KREG + 0xC) as usize;
        track_point(play, &mut self.actor, &mut self.interact_info, preset, NPC_TRACKING_FULL_BODY);
        self.actor.world_rot.y = self.actor.shape_rot.y;
    }

    /// `EnRu1_PlaySittingSfx`: frame 11.
    fn play_sitting_sfx(&self, play: &mut PlayState) {
        if self.skel.on_frame(11.0) {
            sfx(play, NA_SE_EV_LAND_DIRT);
        }
    }

    /// `EnRu1_BecomingCarrier`: explained, she sits (the box paused).
    fn becoming_carrier(&mut self, play: &mut PlayState) -> bool {
        if play.save.get_inf_table(INFTABLE_RUTO_EXPLAINED_WHY_IN_JABU) {
            self.change(self.anims.sit.clone(), ANIMMODE_ONCE, -8.0);
            play.msg_ctx.msg_mode = MSGMODE_PAUSED;
            self.action = ENRU1_ACTION_SPEAK_JABU_SIT_DOWN;
            self.actor.flags &= !(ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY);
            return true;
        }
        false
    }

    /// `EnRu1_CheckJabuTalk`.
    fn check_jabu_talk(&mut self, is_talking: bool) {
        if is_talking {
            self.action = ENRU1_ACTION_SPEAK_JABU_TALKING;
        }
    }

    /// `EnRu1_EndJabuTalk`: the box closed (unless she sits), explained.
    fn end_jabu_talk(&mut self, play: &mut PlayState) {
        if Self::message_should_advance(play) && !self.becoming_carrier(play) {
            play.with_msg(|m, f| m.close_textbox(f.audio));
            play.save.set_inf_table(INFTABLE_RUTO_EXPLAINED_WHY_IN_JABU);
            self.action = ENRU1_ACTION_SPEAK_JABU_IDLE;
        }
    }

    /// `EnRu1_HoldSittingPose`: seated, she lets Link carry her.
    fn hold_sitting_pose(&mut self, play: &mut PlayState, is_fully_seated: bool) {
        if is_fully_seated {
            let a = self.anims.sitting.clone();
            let end = a.last_frame();
            self.skel.change(a, 1.0, 0.0, end, ANIMMODE_LOOP, 0.0);
            play.with_msg(|m, f| m.close_textbox(f.audio));
            play.save.set_inf_table(INFTABLE_RUTO_LET_LINK_CARRY);
            self.func_80aed6dc(play);
            self.offer_carry(play);
            self.action = ENRU1_ACTION_SITTING_IDLE;
            self.disable_sitting_oc();
        }
    }

    // The sapphire.

    /// `EnRu1_PlayStepSoundNearSapphire`.
    fn play_step_sound_near_sapphire(&self, play: &mut PlayState) {
        let s = &self.skel;
        if s.on_frame(2.0) || s.on_frame(7.0) || s.on_frame(12.0) || s.on_frame(18.0) || s.on_frame(25.0) || s.on_frame(33.0) {
            sfx(play, NA_SE_PL_WALK_GROUND + SURFACE_SFX_OFFSET_JABU);
        }
    }

    /// `EnRu1_PlayAbductedSfx`: `@bug (game)`: `SfxSource_PlaySfxAtFixedWorldPos` is given her
    /// `projectedPos` for a world position.
    fn play_abducted_sfx(&self, play: &mut PlayState) {
        play.sfx_source_play_sfx_at_fixed_world_pos(self.actor.projected_pos, 20, NA_SE_VO_RT_FALL);
    }

    /// `EnRu1_PlayLaughingSfx`: frame 5.
    fn play_laughing_sfx(&self, play: &mut PlayState) {
        if self.skel.on_frame(5.0) {
            sfx(play, NA_SE_VO_RT_LAUGH_0);
        }
    }

    /// `EnRu1_RespondToSapphirePlatformMoving`: the platform at `CAM_SET_NORMAL1`: she looks up and
    /// shudders.
    fn respond_to_sapphire_platform_moving(&mut self, play: &mut PlayState) {
        if self.get_platform_cam_setting(play) == CAM_SET_NORMAL1 {
            self.set_eyes(ENRU1_EYES_UP);
            self.set_mouth(ENRU1_MOUTH_OPEN);
            if self.skel.mode != ANIMMODE_ONCE {
                self.animation_change(self.anims.shutter.clone(), ANIMMODE_ONCE, -8.0, false);
                // EnRu1_PlayStartledSfx.
                sfx(play, NA_SE_VO_RT_THROW);
            }
        }
    }

    /// `EnRu1_LookUpAtSapphire`: frame 60, once (`sReachedForSapphire`).
    fn look_up_at_sapphire(&mut self, play: &mut PlayState) {
        if play.overlay_static::<Statics>(ACTOR_EN_RU1).reached_for_sapphire == 0 {
            let cur_frame = self.skel.cur_frame;
            if cur_frame >= 60.0 {
                self.set_eyes(ENRU1_EYES_UP);
                self.set_mouth(ENRU1_MOUTH_SMILING);
                self.func_80aed57c(play);
                play.overlay_static::<Statics>(ACTOR_EN_RU1).reached_for_sapphire = 1;
            }
        }
    }

    /// `EnRu1_SetupRetrievingSapphireAnimation`: past cue 1, at the cue walking to the sapphire.
    fn setup_retrieving_sapphire_animation(&mut self, play: &PlayState) {
        if Self::check_cue_not_matching_id(play, 1, 3)
            && let Some(cue) = play.cs_ctx.npc_actions[3]
        {
            self.actor.world_pos = cue_start(&cue);
            self.actor.shape_rot.x = cue.rot[0];
            self.actor.world_rot.x = cue.rot[0];
            self.actor.shape_rot.y = cue.rot[1];
            self.actor.world_rot.y = cue.rot[1];
            self.actor.shape_rot.z = cue.rot[2];
            self.actor.world_rot.z = cue.rot[2];
            let a = self.anims.walk_to_and_hold_up_sapphire.clone();
            let end = a.last_frame();
            self.skel.change(a, 1.0, 0.0, end, ANIMMODE_ONCE, 0.0);
            self.start_walking_toward_sapphire();
            self.action = ENRU1_ACTION_RETRIEVING_SAPPHIRE;
            self.draw_config = ENRU1_DRAW_OPA;
            self.actor.shape_shadow_alpha = 255;
        }
    }

    /// `EnRu1_HoldUpSapphire`.
    fn hold_up_sapphire(&mut self, done_walking: bool) {
        if done_walking {
            let a = self.anims.hold_arms_up.clone();
            let end = a.last_frame();
            self.skel.change(a, 1.0, 0.0, end, ANIMMODE_LOOP, 0.0);
            self.stop_walking_toward_sapphire();
            self.action = ENRU1_ACTION_HOLDING_SAPPHIRE;
        }
    }

    /// `EnRu1_BringHandsDown`: past cue 2.
    fn bring_hands_down(&mut self, play: &PlayState) {
        if Self::check_cue_not_matching_id(play, 2, 3) {
            self.change(self.anims.bring_hands_down.clone(), ANIMMODE_ONCE, -8.0);
            self.action = ENRU1_ACTION_DONE_HOLDING_SAPPHIRE;
        }
    }

    /// `EnRu1_KeepArmsDown`.
    fn keep_arms_down(&mut self, arms_are_down: bool) {
        if arms_are_down {
            self.change(self.anims.wait2.clone(), ANIMMODE_LOOP, -8.0);
            self.action = ENRU1_ACTION_MISSION_ACCOMPLISHED;
        }
    }

    /// `EnRu1_EndRetrievalCutscene`: the script over (not on a cutscene layer): the sapphire hers,
    /// her switch flag, the platform's `CAM_SET_NORMAL0`; in this room again.
    fn end_retrieval_cutscene(&mut self, play: &mut PlayState) {
        if !play.is_cutscene_layer() && Self::is_cs_state_idle(play) {
            let cur_room_num = Self::cur_room(play);
            play.save.set_inf_table(INFTABLE_RUTO_HAS_SAPPHIRE);
            play.flags.set_switch(self.get_switch_flag() as i32);
            self.set_platform_cam_setting(play, CAM_SET_NORMAL0);
            self.action = ENRU1_ACTION_READY_TO_GO_HOME;
            self.actor.room = cur_room_num;
        }
    }

    /// `EnRu1_RespondToAbduction`: the platform at `CAM_SET_DUNGEON0`: 0x4048, her cry, hidden.
    fn respond_to_abduction(&mut self, play: &mut PlayState) {
        if self.get_platform_cam_setting(play) == CAM_SET_DUNGEON0 {
            self.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY;
            self.actor.text_id = 0x4048;
            // (!OOT_PAL_N64: Message_ContinueTextbox.)
            play.continue_textbox(self.actor.text_id);
            self.play_abducted_sfx(play);
            self.action = ENRU1_ACTION_BEING_ABDUCTED;
            self.draw_config = ENRU1_DRAW_NOTHING;
        }
    }

    /// `EnRu1_RemoveInAbduction`: the box closing, `CAM_SET_DUNGEON1` (Big Octo comes) and gone.
    fn remove_in_abduction(&mut self, play: &mut PlayState) {
        if Self::is_textbox_closing(play) {
            self.set_platform_cam_setting(play, CAM_SET_DUNGEON1);
            self.actor.kill();
        }
    }

    /// `EnRu1_InitInSapphireRoom` (type 4).
    fn init_in_sapphire_room(&mut self, play: &mut PlayState) {
        if play.save.get_inf_table(INFTABLE_RUTO_HAS_SAPPHIRE) && !play.save.get_inf_table(INFTABLE_RUTO_ABDUCTED) {
            self.animation_change(self.anims.wait2.clone(), ANIMMODE_LOOP, 0.0, false);
            self.action = ENRU1_ACTION_WAITING_IN_SAPPHIRE_ROOM;
            self.big_octo_platform = Self::find_big_octo_platform(play);
            self.set_platform_cam_setting(play, CAM_SET_NORMAL0);
            self.actor.flags &= !(ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY);
        } else {
            self.actor.kill();
        }
    }

    // Type 5: beside King Zora.

    /// `EnRu1_InitBesideKingZora`: after her warp, Link a child.
    fn init_beside_king_zora(&mut self, play: &PlayState) {
        if play.save.get_event_chk_inf(EVENTCHKINF_37) && !play.save.adult {
            self.animation_change(self.anims.wait2.clone(), ANIMMODE_LOOP, 0.0, false);
            self.actor.flags &= !ACTOR_FLAG_UPDATE_CULLING_DISABLED;
            self.action = ENRU1_ACTION_THRONE_IDLE;
            self.draw_config = ENRU1_DRAW_OPA;
        } else {
            self.actor.kill();
        }
    }

    /// `EnRu1_ThroneRoomTalkOfferAccepted`: else the mask's line or 0x402C offered.
    fn throne_room_talk_offer_accepted(&mut self, play: &mut PlayState) -> bool {
        if !process_talk_request(&mut self.actor) {
            self.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY;
            self.actor.text_id = Self::throne_room_text_id(play);
            let a = self.actor.clone();
            offer_talk_default(play, &a);
            return false;
        }
        true
    }

    /// The throne room's line: the mask's (`MASK_REACTION_SET_RUTO`), else 0x402C.
    pub fn throne_room_text_id(play: &PlayState) -> u16 {
        let text_id = mask_reaction_get_text_id(play, MASK_REACTION_SET_RUTO);
        if text_id == 0 { 0x402C } else { text_id }
    }

    /// `EnRu1_IsThroneRoomTextClosing`.
    fn is_throne_room_text_closing(&mut self, play: &PlayState) -> bool {
        if play.message_state() == TEXT_STATE_CLOSING {
            self.actor.flags &= !(ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY);
            return true;
        }
        false
    }

    /// `EnRu1_InitBesideDoorSwitch` (type 6).
    fn init_beside_door_switch(&mut self, play: &PlayState) {
        let s = &play.save;
        if s.get_inf_table(INFTABLE_RUTO_MET_FIRST_TIME) && s.get_inf_table(INFTABLE_RUTO_PLACED_ON_SWITCH) && !s.get_inf_table(INFTABLE_RUTO_HAS_SAPPHIRE) && !self.is_assisting_link(play) {
            self.speakable_pre_spawn_init(true);
            log::debug!("En_Ru1: Ruto switch set!!!!!!!!!!!!!!!!!!!!!!");
        } else {
            log::debug!("En_Ru1: Ruto switch not set!!!!!!!!!!!!!!!!!!!!!!");
            self.actor.kill();
        }
    }

    /// `func_80AF0050` (`DEBUG_FEATURES`, type 10).
    fn func_80af0050(&mut self, play: &PlayState) {
        self.animation_change(self.anims.wait2.clone(), ANIMMODE_LOOP, 0.0, false);
        self.action = ENRU1_ACTION_THROWN_ONTO_SAPPHIRE_PLATFORM;
        self.room_num1 = self.actor.room;
        self.big_octo_platform = Self::find_big_octo_platform(play);
        self.actor.room = -1;
    }

    /// `sActionFuncs[action]`.
    fn run_action(&mut self, play: &mut PlayState) {
        match self.action {
            // EnRu1_Fountain_GazingAtLink.
            ENRU1_ACTION_FOUNTAIN_GAZING_AT_LINK => {
                self.init_position_from_fountain_cue(play);
                self.update_skel_anime();
                Self::play_giggle_sfx(play);
                Self::play_link_falling_sfx(play);
                self.enter_fountain_water(play);
            }
            // EnRu1_Fountain_Diving.
            ENRU1_ACTION_FOUNTAIN_DIVING => {
                self.play_diving_entry_sfx(play);
                self.resurface(play);
            }
            // EnRu1_Fountain_Resurfacing.
            ENRU1_ACTION_FOUNTAIN_RESURFACING => {
                let anim_finished = self.update_skel_anime();
                self.update_bg_check_info(play);
                self.play_resurfacing_sfx(play);
                self.resurface_progress(anim_finished);
            }
            // EnRu1_Fountain_TreadingWater.
            ENRU1_ACTION_FOUNTAIN_TREADING_WATER => {
                self.swim_toward_link(play);
                self.update_bg_check_info(play);
                self.update_skel_anime();
                self.spawn_ripple_treading(play);
                self.stop_treading(play);
            }
            // EnRu1_Fountain_StartingSwimBack.
            ENRU1_ACTION_FOUNTAIN_STARTING_SWIM_BACK => {
                let anim_finished = self.update_skel_anime();
                self.update_bg_check_info(play);
                self.update_eyes(play);
                self.spawn_ripple_treading(play);
                self.start_back_swimming(anim_finished);
                self.play_submerge_sfx(play);
                self.transition_to_back_swimming(play);
            }
            // EnRu1_Fountain_SwimmingBack.
            ENRU1_ACTION_FOUNTAIN_SWIMMING_BACK => {
                let anim_finished = self.update_skel_anime();
                self.swim_away_from_link(play);
                self.update_bg_check_info(play);
                self.update_eyes(play);
                self.spawn_ripple_treading(play);
                self.start_back_swimming(anim_finished);
                self.play_swim_stroke_sfx(play);
                self.end_swim_back(play);
            }
            // EnRu1_Fountain_FinishingSwimBack.
            ENRU1_ACTION_FOUNTAIN_FINISHING_SWIM_BACK => {
                let anim_finished = self.update_skel_anime();
                self.update_bg_check_info(play);
                self.update_eyes(play);
                self.spawn_ripple_treading(play);
                self.end_giving_sapphire(play, anim_finished);
            }
            // EnRu1_Meeting_RangeCheck.
            ENRU1_ACTION_MEETING_RANGE_CHECK => self.check_start_first_encounter(play),
            // EnRu1_Meeting_InitPosition.
            ENRU1_ACTION_MEETING_INIT_POSITION => self.setup_position_for_first_encounter(play),
            // EnRu1_Meeting_FacingLink.
            ENRU1_ACTION_MEETING_FACING_LINK => {
                self.update_skel_anime();
                self.update_eyes(play);
                self.update_bg_check_info(play);
                self.check_turn_around(play);
            }
            // EnRu1_Meeting_TurningAround.
            ENRU1_ACTION_MEETING_TURNING_AROUND => {
                let anim_finished = self.update_skel_anime();
                self.update_eyes(play);
                self.update_bg_check_info(play);
                self.start_walking_away(anim_finished);
            }
            // EnRu1_Meeting_WalkingAwayAccel.
            ENRU1_ACTION_MEETING_WALKING_AWAY_ACCEL => {
                self.accelerate_away();
                self.update_skel_anime();
                self.update_eyes(play);
                self.update_bg_check_info(play);
                self.play_step_sfx(play);
                self.advance_walking_away();
            }
            // EnRu1_Meeting_WalkingAwayConstant.
            ENRU1_ACTION_MEETING_WALKING_AWAY_CONSTANT => {
                self.move_forward_constant();
                self.respond_to_falling(play);
                self.update_skel_anime();
                self.update_eyes(play);
                self.update_bg_check_info(play);
                self.play_step_sfx(play);
                self.check_start_falling();
            }
            // EnRu1_Meeting_FallingDownHole.
            ENRU1_ACTION_MEETING_FALLING_DOWN_HOLE => {
                self.accelerate_down_hole();
                self.respond_to_falling(play);
                self.update_skel_anime();
                self.update_eyes(play);
                self.update_bg_check_info(play);
                self.play_step_sfx(play);
                self.finish_first_encounter(play);
            }
            // EnRu1_Meeting_End.
            ENRU1_ACTION_MEETING_END => {
                if play.cs_ctx.state == CS_STATE_IDLE {
                    self.actor.kill();
                }
            }
            // EnRu1_BossRoom_PreSpawn (EnRu1_TriggerSpawnInBossRoom).
            ENRU1_ACTION_PRE_SPAWN_BOSS_ROOM => self.action = ENRU1_ACTION_SPAWN_BOSS_ROOM,
            // EnRu1_BossRoom_Spawn.
            ENRU1_ACTION_SPAWN_BOSS_ROOM => self.setup_rise_through_blue_warp(play),
            // EnRu1_BossRoom_RiseThroughBlueWarp.
            ENRU1_ACTION_RISE_THROUGH_BLUE_WARP => {
                self.rise();
                self.update_skel_anime();
                self.end_rise(play);
            }
            // EnRu1_BossRoom_WaitInsideBlueWarp.
            ENRU1_ACTION_WAIT_INSIDE_BLUE_WARP => {
                self.update_skel_anime();
                self.check_link_entered_blue_warp(play);
            }
            // EnRu1_BossRoom_LinkWalksToPointInBlueWarp.
            ENRU1_ACTION_LINK_WALKS_TO_POINT_IN_BLUE_WARP => {
                self.update_skel_anime();
                let in_warp = self.is_link_in_blue_warp(play);
                self.start_crossing_arms_and_legs(play, in_warp);
            }
            // EnRu1_BossRoom_WhatTookYouSoLong.
            ENRU1_ACTION_WHAT_TOOK_YOU_SO_LONG => {
                self.update_bg_check_info(play);
                let is_talking = self.update_skel_anime();
                self.advance_angry_animation(play, is_talking);
            }
            // EnRu1_BossRoom_WarpingOut.
            ENRU1_ACTION_WARPING_OUT => {
                self.rise_with_link(play);
                self.update_bg_check_info(play);
                self.update_skel_anime();
            }
            // EnRu1_SpeakableJabu_PreSpawn.
            ENRU1_ACTION_SPEAK_JABU_PRE_SPAWN => self.action = ENRU1_ACTION_SPEAK_JABU_SPAWN,
            // EnRu1_SpeakableJabu_Spawn.
            ENRU1_ACTION_SPEAK_JABU_SPAWN => {
                self.action = ENRU1_ACTION_SPEAK_JABU_IDLE;
                self.draw_config = ENRU1_DRAW_OPA;
                self.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_FRIENDLY;
            }
            // EnRu1_SpeakableJabu_Idle.
            ENRU1_ACTION_SPEAK_JABU_IDLE => {
                self.turn_upper_body_toward_player(play);
                self.update_skel_anime();
                self.update_eyes(play);
                self.update_standing_oc(play);
                self.update_bg_check_info(play);
                let is_talking = self.talk_offer_accepted(play);
                self.func_80aed624(play);
                self.check_jabu_talk(is_talking);
            }
            // EnRu1_SpeakableJabu_Talking.
            ENRU1_ACTION_SPEAK_JABU_TALKING => {
                self.turn_full_body_toward_player(play);
                self.update_skel_anime();
                self.update_eyes(play);
                self.update_bg_check_info(play);
                self.end_jabu_talk(play);
            }
            // EnRu1_SpeakableJabu_TakingSeat.
            ENRU1_ACTION_SPEAK_JABU_SIT_DOWN => {
                self.turn_upper_body_forward();
                let anim_finished = self.update_skel_anime();
                self.play_sitting_sfx(play);
                self.update_eyes(play);
                self.update_bg_check_info(play);
                self.hold_sitting_pose(play, anim_finished);
            }
            // EnRu1_Sitting_Idle.
            ENRU1_ACTION_SITTING_IDLE => {
                self.turn_upper_body_forward();
                self.update_sitting_oc(play);
                self.update_bg_check_info(play);
                self.update_skel_anime();
                self.update_eyes(play);
                self.offer_carry(play);
                self.update_sitting_action(play);
                self.func_80aed624(play);
                self.reset_bg_check_flags(play);
            }
            // EnRu1_Sitting_Released.
            ENRU1_ACTION_SITTING_RELEASED => {
                self.turn_upper_body_forward();
                self.update_sitting_at(play);
                self.update_bg_check_info(play);
                self.check_for_door_switch(play);
                self.update_position(play);
                self.update_skel_anime();
                self.update_eyes(play);
                self.check_landing(play);
                self.func_80aed624(play);
                self.reset_bg_check_flags(play);
            }
            // EnRu1_Sitting_EnteringWater.
            ENRU1_ACTION_SITTING_ENTERING_WATER => {
                self.turn_upper_body_forward();
                self.update_bg_check_info(play);
                self.update_water_state();
                self.update_skel_anime();
                self.update_eyes(play);
                self.check_sinking_state(play);
                self.func_80aed624(play);
            }
            // EnRu1_Sitting_SinkingInWater.
            ENRU1_ACTION_SITTING_SINKING => {
                self.turn_upper_body_forward();
                self.update_bg_check_info(play);
                self.actor.move_forward();
                self.update_skel_anime();
                self.update_eyes(play);
                self.check_hit_bottom_underwater(play);
                self.func_80aed624(play);
                self.reset_bg_check_flags(play);
            }
            // EnRu1_Sitting_Carried.
            ENRU1_ACTION_SITTING_CARRIED => {
                self.update_head_rotation(play);
                self.update_skel_anime();
                self.update_bg_check_info(play);
                self.check_for_door_switch(play);
                self.update_eyes(play);
                Self::check_if_back_in_holes_room(play);
                self.update_carried_behavior(play);
            }
            // EnRu1_Sitting_LosingBalance.
            ENRU1_ACTION_SITTING_LOSING_BALANCE => {
                self.turn_upper_body_forward();
                self.update_skel_anime();
                self.update_bg_check_info(play);
                self.check_for_door_switch(play);
                self.update_eyes(play);
                Self::check_if_back_in_holes_room(play);
                self.update_carried_behavior(play);
            }
            // EnRu1_Sitting_DisappearingInWater.
            ENRU1_ACTION_SITTING_DISAPPEARING => {
                self.turn_upper_body_forward();
                self.update_bg_check_info(play);
                self.update_position(play);
                self.update_skel_anime();
                self.update_eyes(play);
                self.fade_while_sinking(play);
                self.func_80aed624(play);
            }
            // EnRu1_Sitting_SeesSapphire.
            ENRU1_ACTION_SITTING_SEES_SAPPHIRE => {
                self.turn_upper_body_forward();
                self.update_skel_anime();
                self.update_bg_check_info(play);
                self.update_eyes(play);
                self.end_seeing_sapphire_animation(play);
            }
            // EnRu1_Action_Unused.
            ENRU1_ACTION_UNUSED => {}
            // EnRu1_SapphireRoom_ThrownOntoPlatform.
            ENRU1_ACTION_THROWN_ONTO_SAPPHIRE_PLATFORM => {
                self.turn_upper_body_forward();
                self.update_skel_anime();
                self.update_eyes(play);
                self.setup_retrieving_sapphire_animation(play);
                self.func_80aeb220(play);
            }
            // EnRu1_SapphireRoom_Retrieving.
            ENRU1_ACTION_RETRIEVING_SAPPHIRE => {
                self.turn_upper_body_forward();
                self.walk_toward_sapphire();
                self.update_bg_check_info(play);
                let anim_finished = self.update_skel_anime();
                self.play_laughing_sfx(play);
                self.look_up_at_sapphire(play);
                self.play_step_sound_near_sapphire(play);
                self.hold_up_sapphire(anim_finished);
                self.func_80aeb220(play);
            }
            // EnRu1_SapphireRoom_Holding.
            ENRU1_ACTION_HOLDING_SAPPHIRE => {
                self.turn_upper_body_forward();
                self.update_bg_check_info(play);
                self.update_skel_anime();
                self.bring_hands_down(play);
                self.func_80aeb220(play);
            }
            // EnRu1_SapphireRoom_DoneHolding.
            ENRU1_ACTION_DONE_HOLDING_SAPPHIRE => {
                self.turn_upper_body_forward();
                self.update_bg_check_info(play);
                let anim_finished = self.update_skel_anime();
                self.update_eyes(play);
                self.keep_arms_down(anim_finished);
                self.func_80aeb220(play);
            }
            // EnRu1_SapphireRoom_MissionAccomplished.
            ENRU1_ACTION_MISSION_ACCOMPLISHED => {
                self.turn_upper_body_forward();
                self.update_bg_check_info(play);
                self.update_skel_anime();
                self.update_eyes(play);
                self.end_retrieval_cutscene(play);
                self.func_80aeb220(play);
            }
            // EnRu1_SapphireRoom_Waiting.
            ENRU1_ACTION_WAITING_IN_SAPPHIRE_ROOM => {
                self.big_octo_platform = Self::find_big_octo_platform(play);
                if self.big_octo_platform.is_some() {
                    self.action = ENRU1_ACTION_READY_TO_GO_HOME;
                    self.draw_config = ENRU1_DRAW_OPA;
                    self.set_platform_cam_setting(play, CAM_SET_NORMAL0);
                }
            }
            // EnRu1_SapphireRoom_ReadyToGoHome.
            ENRU1_ACTION_READY_TO_GO_HOME => {
                self.turn_upper_body_forward();
                self.update_bg_check_info(play);
                self.update_skel_anime();
                self.update_eyes(play);
                self.respond_to_sapphire_platform_moving(play);
                self.respond_to_abduction(play);
            }
            // EnRu1_SapphireRoom_BeingAbducted.
            ENRU1_ACTION_BEING_ABDUCTED => {
                self.turn_upper_body_forward();
                self.remove_in_abduction(play);
            }
            // EnRu1_ThroneRoom_Idle.
            ENRU1_ACTION_THRONE_IDLE => {
                self.turn_upper_body_toward_player(play);
                self.update_skel_anime();
                self.update_eyes(play);
                self.update_standing_oc(play);
                self.update_bg_check_info(play);
                // EnRu1_StartThroneRoomTalk.
                if self.throne_room_talk_offer_accepted(play) {
                    self.action = ENRU1_ACTION_THRONE_TALK;
                }
            }
            // EnRu1_ThroneRoom_Talk.
            ENRU1_ACTION_THRONE_TALK => {
                self.turn_full_body_toward_player(play);
                self.update_skel_anime();
                self.update_eyes(play);
                self.update_bg_check_info(play);
                // EnRu1_EndThroneRoomTalk.
                if self.is_throne_room_text_closing(play) {
                    self.action = ENRU1_ACTION_THRONE_IDLE;
                }
            }
            _ => {}
        }
    }

    /// Her skeleton posed as `EnRu1_OverrideLimbDraw` turns it (`EnRu1_PreLimbDraw`): the chest by
    /// the torso's turn, the head by the head's.
    fn pose(skeleton: &Skeleton, joints: &[[i16; 3]], head: [i16; 3], torso: [i16; 3]) -> Vec<Mat4> {
        skeleton.pose_override(joints, |limb, _pos, rot| {
            match limb {
                RUTO_CHILD_CHEST => {
                    rot[0] = rot[0].wrapping_add(torso[1]);
                    rot[1] = rot[1].wrapping_sub(torso[0]);
                }
                RUTO_CHILD_HEAD => {
                    rot[0] = rot[0].wrapping_add(head[1]);
                    rot[2] = rot[2].wrapping_add(head[0]);
                }
                _ => {}
            }
            Mat4::IDENTITY
        })
    }
}

/// Indices into the render state's switches.
mod rs {
    pub const DRAW_CONFIG: usize = 0;
    pub const EYES: usize = 1;
    pub const MOUTH: usize = 2;
    pub const ALPHA: usize = 3;
    pub const SWITCHES: usize = 4;
}

impl ActorImpl for EnRu1 {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnRu1_Update`.
    fn update(&mut self, play: &mut PlayState) {
        if !(0..ACTION_FUNCS).contains(&self.action) {
            if !self.logged {
                log::debug!("En_Ru1: The main mode is wrong!!!!!!!!!!!!!!!!!!!!!!!!!");
                self.logged = true;
            }
            return;
        }
        self.run_action(play);
    }

    /// `AnimTask_ActorMovement` for her queued move: her root's motion, at her scale.
    fn animation_update(&mut self) {
        if !std::mem::take(&mut self.movement_queued) {
            return;
        }
        let diff = self.skel.update_translation(self.actor.shape_rot.y);
        self.actor.world_pos.x += diff.x * self.actor.scale.x;
        self.actor.world_pos.y += diff.y * self.actor.scale.y;
        self.actor.world_pos.z += diff.z * self.actor.scale.z;
    }

    /// `EnRu1_PostLimbDraw` (the opaque draw's): the focus 10 above her head.
    fn draw_update(&mut self, _play: &mut PlayState) {
        if self.draw_config != ENRU1_DRAW_OPA {
            return;
        }
        let bones = Self::pose(&self.skeleton, &self.skel.joint_table, self.interact_info.head, self.interact_info.torso);
        if let Some(head) = bones.get(RUTO_CHILD_HEAD - 1) {
            let m = actor_draw_matrix(&RenderState::of(&self.actor));
            self.actor.focus_pos = (m * *head).transform_point3(FOCUS_OFFSET);
            self.actor.focus_rot = self.actor.world_rot;
        }
    }

    fn render_state(&self) -> RenderState {
        let mut r = RenderState::of(&self.actor);
        r.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        let (h, t) = (self.interact_info.head, self.interact_info.torso);
        r.angles = vec![h[0], h[1], h[2], t[0], t[1], t[2]];
        r.switches = vec![self.draw_config as u32, self.eyes as u32, self.mouth as u32, (self.alpha as u32) & 0xFF];
        r
    }

    /// `EnRu1_Draw`: by `drawConfig`, nothing, `EnRu1_DrawOpa` (env alpha 255) or `EnRu1_DrawXlu`
    /// (env alpha `alpha`): the eyes on segment 8, the mouth on 9.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        if rs.switches.len() < rs::SWITCHES || rs.angles.len() < 6 {
            return;
        }
        let draw_config = rs.switches[rs::DRAW_CONFIG] as i32;
        let (bake, xlu, alpha) = match draw_config {
            ENRU1_DRAW_OPA => (BAKE_OPA, false, 255u8),
            ENRU1_DRAW_XLU => (BAKE_XLU, true, rs.switches[rs::ALPHA] as u8),
            ENRU1_DRAW_NOTHING => return,
            _ => {
                log::debug!("En_Ru1: The drawing mode is wrong!!!!!!!!!!!!!!!!!!!!!!!!!");
                return;
            }
        };
        let Some(joints) = &rs.joints else { return };
        let a = &rs.angles;
        let bones = Self::pose(&self.skeleton, &joints.rot, [a[0], a[1], a[2]], [a[3], a[4], a[5]]);
        let eyes = EYE_TEXTURES.len();
        let eye = self.face[(rs.switches[rs::EYES] as usize).min(eyes - 1)].clone();
        let mouth = self.face[eyes + (rs.switches[rs::MOUTH] as usize).min(MOUTH_TEXTURES.len() - 1)].clone();
        let texture_images = vec![SourceImage { source: SEG_EYES, image: eye }, SourceImage { source: SEG_MOUTH, image: mouth }];
        let mut sv = SegmentValues::default();
        sv.env[SEG_ALPHA as usize] = Some([0, 0, 0, alpha]);
        let cmd = DrawCmd { mesh: MeshKey::named(keys::bake(bake)), transform: actor_draw_matrix(rs), bones, params: DrawParams { segments: Some(sv), texture_images, ..Default::default() } };
        if xlu {
            out.xlu.push(cmd);
        } else {
            out.opa.push(cmd);
        }
    }

    /// `EnRu1_Destroy` (`EnRu1_DestroyColliders`).
    fn destroy(&mut self, _play: &mut PlayState) {}

    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        match id {
            COLLIDER_STANDING => Some(ColliderMut::Cylinder(&mut self.standing_collider)),
            COLLIDER_SITTING => Some(ColliderMut::Cylinder(&mut self.sitting_collider)),
            _ => None,
        }
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
