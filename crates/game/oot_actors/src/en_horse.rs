//! `En_Horse` (`ovl_En_Horse/z_en_horse.c`): the horses Link rides. Epona (`HORSE_EPONA`,
//! `object_horse`, `gEponaSkel`) or the rental horse Ingo also races on (`HORSE_HNI`, params bit
//! 15, `object_hni`, `gHorseIngoSkel`): a skin skeleton (`oot_game::skin`) playing nine animations
//! by type (idle, whinny, refuse, rearing, walking, trotting, galloping, jumping, jumping high).
//!
//! **The init** reads params (`HorseParamType` once bit 15 is taken off; -1 becomes
//! `HORSE_PTYPE_1`):
//! - 2 `INACTIVE` (`z_horse.c`'s spawn at a scene's song place): not drawn, waiting for Epona's
//!   Song (`R_EPONAS_SONG_PLAYED`, the message box's) to appear at the nearest of
//!   `sHorseSpawns`' places out of view and come to Link (`EnHorse_Spawn`, the camera turned on
//!   her: `CAM_SET_TURN_AROUND`).
//! - 3 `INGO_SPAWNED_RIDING` (the race's `En_Horse_Game_Check`): Ingo's horse, spawning Ingo on it
//!   (`En_In` 1); it runs the race's eight waypoints (`EnHorse_UpdateIngoRace`).
//! - 7: a cutscene's (`z_horse.c`'s jumps over the ranch's fences, the credits): it follows the
//!   script's player cues 36 to 38, 64 and 65 (move, jump, rear, warp-move, warp-rear).
//! - 8 `HORSEBACK_ARCHERY` (the Gerudo's Fortress, Phase 18): logged; it stands.
//! - On the ranch without Epona (`EVENTCHKINF_EPONA_OBTAINED`): she runs from Link
//!   (`EnHorse_FleePlayer`) and goes home, unrideable, until Epona's Song; the rental horse is
//!   rideable at once.
//! - Else idle and rideable (`EnHorse_StartIdleRidable`); in the stable, and the Gerudo's
//!   Fortress's rental horse, idle and unrideable.
//!
//! On the ranch (not a cutscene layer) the rental horse is there only by day, before Epona and with
//! `rot.z` 5; Epona only by day before she's had. In the stable a horse is there only by night,
//! for an adult, before Epona.
//!
//! **Mounting** (`EnHorse_MountDismount`): standing idle or whinnying, with Link within 75 across
//! and 30 up or down, facing her side (within 80 degrees) from her left or right
//! (`EnHorse_GetMountSide`), she offers herself (`Actor_SetRideActor`); Player's A
//! (`Player_ActionHandler_3`) mounts her (`Actor_MountHorse`: her `child` is Link). Mounted, she
//! freezes 55 frames, then takes the stick (`EnHorse_UpdateStick`, `EnHorse_StickDirection`):
//! idle, turning, walking (to 3), trotting (to 6), galloping (to 8, `EnHorse_UpdateSpeed`), rearing,
//! stopping (the stick pulled back at a gallop), reversing. A spurs her (`EnHorse_CheckBoost`, the
//! A button "Faster", `interfaceCtx.unk_1EE` 8): six carrots to 14 (Epona) or 12, regained in
//! turn (`EnHorse_RegenBoost`, the HUD's carrots, `numHorseBoosts`). Dismounted she freezes 35.
//!
//! **Jumps** (`EnHorse_UpdateBgCheckInfo`): at more than 8 a line test 19 up, 160 ahead (230 from
//! 12.8) finds a wall; under 80 (150) away she brakes; otherwise the ground beyond it is measured:
//! 19 to 40 high at a gallop under 12.8 is a low jump, 19 to 72 at 12.8 to 13.8 (or up to 112 above
//! 13.8) a high one, started from the draw (`postDrawFunc`). On Malon's course (`ENHORSE_FLAG_26`)
//! only a `Bg_Umajump` can be jumped. Slower, the wall 30 ahead pushes her off
//! (`EnHorse_BgCheckSlowMoving`). The floors under her front and back (`EnHorse_CheckFloors`,
//! 30 each way, 60 up) pitch her; water, a drop, a slope over 35 degrees, a horse-blocked or
//! `FLOOR_TYPE_7` floor stops her (`EnHorse_ObstructMovement`: rearing at a gallop).
//!
//! **The draw** (`EnHorse_Draw`, not while inactive): the skin (`func_800A6360`, without its
//! root's translation while jumping); the eyes on limb 13 (`EnHorse_OverrideLimbDraw`, Epona's
//! three by `blinkTimer`). The post-draw (`EnHorse_PostDraw`, `draw_update` here): the rider's
//! place on limb 30 (`riderPos`, which Player rides at), the head on limb 13 (`unk_228`, her
//! voice's), the hooves' dust by animation and frame (with `Rand`), the head's sphere (OC and AC,
//! `@bug (game)`: from the draw). Then a jump `postDrawFunc` asked for starts.
//!
//! Every frame: the stick, Link's side, mounting; the action; the boosts; `Actor_MoveXZGravity`;
//! Ingo carried at 10 up; slowed by an OC hit of the head, a neigh at an AC hit; the roll in turns
//! (`EnHorse_TiltBody`); the cylinders (the front's AT, at 5 and up, knocks enemies down); the
//! conveyors (riding through an exit); the floors; the focus 70 up; the blink (a 1 in 40 chance);
//! immovable standing; the hooves' dust (`func_800287AC`, not on the ranch's credits layer 9).
//!
//! Ported whole but: horseback archery (`EnHorse_InitHorsebackArchery`,
//! `EnHorse_UpdateHorsebackArchery`, `EnHorse_UpdateHbaRaceInfo`, `EnHorse_UpdateHbaAnim`: the
//! Gerudo's Fortress, Phase 18: logged), the Gerudo Valley bridge jump's start
//! (`EnHorse_StartBridgeJump`'s scripts, Phase 18: logged; the jump's actions and checks are
//! ported), the Gerudo saddle (`gHorseIngoGerudoSaddleDL` on the Fortress's rental horse,
//! `ENHORSE_FLAG_18`: Phase 18) and the rumble (`Rumble_Request`: no rumble here). The sounds the C
//! plays at the head (`unk_21C`) play at the horse (BACKLOG #68).
//!
//! Params: `params` bit 15 the type (0 Epona, 1 the rental horse, `HORSE_HNI`), bits 0 to 14 the
//! `HorseParamType` (0x7FFF placed: `HORSE_PTYPE_1`; 2 inactive, 3 Ingo's, 5 trapped, 6 the race,
//! 7 a cutscene's, 8 horseback archery, 9 ridden into the scene). `world.rot.z`: on the ranch, the
//! rental horse's day (5 before Epona), read then zeroed.
//! Flags: reads EVENTCHKINF_EPONA_OBTAINED (the ranch, the stable, the course), EVENTINF's
//! `INGO_RACE_STATE` (trapped), `_SECOND_RACE` (Ingo's speed, the rental horse's rear) and
//! `R_EPONAS_SONG_PLAYED` (cleared), EVENTCHKINF_CARPENTERS_ALL_RESCUED (the bridge jump); clears
//! `R_EXITED_SCENE_RIDING_HORSE` at its init; writes `horseData.sceneId` when called.

use std::sync::Arc;

use eng_anim::anim::JointTable;
use eng_collision::bgcheck::{BGCHECK_Y_MIN, PolyId, dist_plane_to_pos};
use eng_collision::math3d::{Cylinder16, Sphere16};
use eng_gfx::{DrawCmd, DrawImage, DrawParams, MeshKey, SourceImage};
use eng_math::{cos_s, sin_s};
use glam::{Vec2, Vec3};
use oot_game::actor::{ACTOR_FLAG_TALK, ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor, BGCHECKFLAG_WALL, UPDBGCHECKINFO_FLAG_0, UPDBGCHECKINFO_FLAG_2, UPDBGCHECKINFO_FLAG_3, UPDBGCHECKINFO_FLAG_4};
use oot_game::actor_ctx::{ACTORCAT_BG, ActorHandle, ActorImpl, ActorProfile, audio_play_actor_sfx2};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind, ObjectNeed, Source};
use oot_game::audio::sfx::*;
use oot_game::camera::{CAM_SET_TURN_AROUND, f_atan2f};
use oot_game::collision_check::*;
use oot_game::cutscene::{CS_STATE_IDLE, CsCmdActorCue};
use oot_game::event_chk_inf;
use oot_game::horse::*;
use oot_game::pack::{BakeBody, MeshBake, keys};
use oot_game::path::rad_to_binang_trunc;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::skelanime_std::{ANIMMODE_LOOP, ANIMMODE_ONCE, Anim, SkelAnimeStd};
use oot_game::skin::{SkinPlace, SkinSkeleton, apply_anim_transformations, get_limb_pos, skin_draw};
use oot_game::surface::SurfaceType;
use oot_game::target::yaw_to;

/// `En_Horse_Profile`: `ACTORCAT_BG`, `ACTOR_FLAG_UPDATE_CULLING_DISABLED`, `OBJECT_HORSE`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_HORSE, name: "En_Horse", category: ACTORCAT_BG, flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED, object: OBJECT, schema: &SCHEMA };
/// `OBJECT_HORSE` (`object_table.h`: 0x001A), `OBJECT_HNI` (0x00D2).
pub const OBJECT: &str = "object_horse";
pub const OBJECT_HNI: &str = "object_hni";
const OBJECT_HNI_ID: i16 = 0x00D2;

/// `HorseType`.
pub const HORSE_EPONA: i32 = 0;
pub const HORSE_HNI: i32 = 1;

/// `HorseParamType` (the ones `oot_game::horse` doesn't name).
pub const HORSE_PTYPE_0: i16 = 0;
pub const HORSE_PTYPE_INGO_SPAWNED_RIDING: i16 = 3;
pub const HORSE_PTYPE_4: i16 = 4;
pub const HORSE_PTYPE_10: i16 = 10;

/// `params` bit 15: the type.
pub const TYPE: Field = Field::params("type", 15, 1, Kind::Named(&[(0, "Epona (object_horse)"), (1, "the rental horse, Ingo's (object_hni)")]), "which horse");
/// Bits 0 to 14: `HorseParamType`.
pub const PTYPE: Field = Field::params(
    "params & 0x7FFF",
    0,
    15,
    Kind::Named(&[
        (0, "0 (played: after a freeze)"),
        (1, "idle, rideable (0x7FFF placed)"),
        (2, "inactive: comes to Epona's Song"),
        (3, "Ingo's in the race (spawns Ingo)"),
        (4, "4"),
        (5, "Link on it, shut in the ranch with Epona won"),
        (6, "Link on it at the race's start"),
        (7, "a cutscene's (the ranch fence jumps, the credits)"),
        (8, "horseback archery (Phase 18)"),
        (9, "Link riding it into the scene"),
        (0x7FFF, "placed -1: as 1"),
    ]),
    "what the horse does from its start",
);
/// `world.rot.z`: on the ranch, the rental horse's.
pub const ROT_Z: Field = Field::rot("rot.z", Source::RotZ, 0, 16, Kind::Named(&[(0, "never on the ranch"), (5, "the ranch's rental horse, by day before Epona")]), "on the ranch the rental horse is there with 5 (before Epona, by day); zeroed by the init");

pub const EVENTCHKINF_CARPENTERS_ALL_RESCUED_FLAGS: [u16; 4] = [0x90, 0x91, 0x92, 0x93];

/// En_Horse's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[TYPE, PTYPE, ROT_Z],
    flags: &[
        FlagUse::new(event_chk_inf!(EVENTCHKINF_EPONA_OBTAINED)).reads().note("the ranch: Epona flees without it; the stable's horse; the course's jumps"),
        FlagUse::new(Flag::Other("EVENTINF_INGO_RACE_STATE")).reads().note("trapped with Epona won (6): she won't be called elsewhere in the ranch"),
        FlagUse::new(oot_game::event_inf!(EVENTINF_INGO_RACE_SECOND_RACE)).reads().note("Ingo's speed; the rental horse rears at the second race's start"),
        FlagUse::new(Flag::Other("R_EPONAS_SONG_PLAYED")).reads().clears().note("Epona's Song: she comes, and stops fleeing"),
        FlagUse::new(Flag::Other("R_EXITED_SCENE_RIDING_HORSE")).clears().note("its init"),
        FlagUse::new(Flag::Other("R_DEBUG_FORCE_EPONA_OBTAINED")).reads().note("the debug register DREG(1): Epona's as obtained (0)"),
        FlagUse::new(Flag::Other("EVENTCHKINF_CARPENTERS_ALL_RESCUED")).reads().note("Gerudo Valley's bridge jump (Phase 18)"),
    ],
    todo: false,
    objects: &[ObjectNeed::new(OBJECT_HNI).when("type", &[1]).note("the rental horse's skeleton and animations: without it, it goes at init")],
    texts: &[],
};

// EnHorseAction.
pub const ENHORSE_ACT_FROZEN: i32 = 0;
pub const ENHORSE_ACT_INACTIVE: i32 = 1;
pub const ENHORSE_ACT_IDLE: i32 = 2;
pub const ENHORSE_ACT_FOLLOW_PLAYER: i32 = 3;
pub const ENHORSE_ACT_INGO_RACE: i32 = 4;
pub const ENHORSE_ACT_MOUNTED_IDLE: i32 = 5;
pub const ENHORSE_ACT_MOUNTED_IDLE_WHINNEYING: i32 = 6;
pub const ENHORSE_ACT_MOUNTED_TURN: i32 = 7;
pub const ENHORSE_ACT_MOUNTED_WALK: i32 = 8;
pub const ENHORSE_ACT_MOUNTED_TROT: i32 = 9;
pub const ENHORSE_ACT_MOUNTED_GALLOP: i32 = 10;
pub const ENHORSE_ACT_MOUNTED_REARING: i32 = 11;
pub const ENHORSE_ACT_STOPPING: i32 = 12;
pub const ENHORSE_ACT_REVERSE: i32 = 13;
pub const ENHORSE_ACT_LOW_JUMP: i32 = 14;
pub const ENHORSE_ACT_HIGH_JUMP: i32 = 15;
pub const ENHORSE_ACT_BRIDGE_JUMP: i32 = 16;
pub const ENHORSE_ACT_CS_UPDATE: i32 = 17;
pub const ENHORSE_ACT_HBA: i32 = 18;
pub const ENHORSE_ACT_FLEE_PLAYER: i32 = 19;

// stateFlags.
pub const ENHORSE_BOOST: u32 = 1 << 0;
pub const ENHORSE_BOOST_DECEL: u32 = 1 << 1;
pub const ENHORSE_JUMPING: u32 = 1 << 2;
pub const ENHORSE_CALC_RIDER_POS: u32 = 1 << 3;
pub const ENHORSE_FORCE_REVERSING: u32 = 1 << 4;
pub const ENHORSE_FORCE_WALKING: u32 = 1 << 5;
pub const ENHORSE_FLAG_6: u32 = 1 << 6;
pub const ENHORSE_FLAG_7: u32 = 1 << 7;
pub const ENHORSE_FLAG_8: u32 = 1 << 8;
pub const ENHORSE_FLAG_9: u32 = 1 << 9;
pub const ENHORSE_STOPPING_NEIGH_SOUND: u32 = 1 << 10;
pub const ENHORSE_LAND2_SOUND: u32 = 1 << 11;
pub const ENHORSE_SANDDUST_SOUND: u32 = 1 << 12;
pub const ENHORSE_INACTIVE: u32 = 1 << 13;
pub const ENHORSE_OBSTACLE: u32 = 1 << 14;
pub const ENHORSE_TURNING_TO_PLAYER: u32 = 1 << 15;
pub const ENHORSE_UNRIDEABLE: u32 = 1 << 16;
pub const ENHORSE_CANT_JUMP: u32 = 1 << 17;
pub const ENHORSE_FLAG_18: u32 = 1 << 18;
pub const ENHORSE_FLAG_19: u32 = 1 << 19;
pub const ENHORSE_FLAG_20: u32 = 1 << 20;
pub const ENHORSE_FLAG_21: u32 = 1 << 21;
pub const ENHORSE_FIRST_BOOST_REGEN: u32 = 1 << 22;
pub const ENHORSE_INGO_WON: u32 = 1 << 23;
pub const ENHORSE_FLAG_24: u32 = 1 << 24;
pub const ENHORSE_FLAG_25: u32 = 1 << 25;
pub const ENHORSE_FLAG_26: u32 = 1 << 26;
pub const ENHORSE_DRAW: u32 = 1 << 27;

// EnHorsePlayerDir.
pub const PLAYER_DIR_FRONT_R: u32 = 0;
pub const PLAYER_DIR_FRONT_L: u32 = 1;
pub const PLAYER_DIR_BACK_R: u32 = 2;
pub const PLAYER_DIR_BACK_L: u32 = 3;
pub const PLAYER_DIR_SIDE_R: u32 = 4;
pub const PLAYER_DIR_SIDE_L: u32 = 5;

// EnHorseAnimationIndex.
pub const ENHORSE_ANIM_IDLE: i32 = 0;
pub const ENHORSE_ANIM_WHINNEY: i32 = 1;
pub const ENHORSE_ANIM_STOPPING: i32 = 2;
pub const ENHORSE_ANIM_REARING: i32 = 3;
pub const ENHORSE_ANIM_WALK: i32 = 4;
pub const ENHORSE_ANIM_TROT: i32 = 5;
pub const ENHORSE_ANIM_GALLOP: i32 = 6;
pub const ENHORSE_ANIM_LOW_JUMP: i32 = 7;
pub const ENHORSE_ANIM_HIGH_JUMP: i32 = 8;

/// `INGO_FLAG_LASHING`.
const INGO_FLAG_LASHING: u16 = 1 << 0;

/// `EnInHorseAnimation` (`z_en_in.h`).
pub const ENIN_HORSE_ANIM_SLOW_RIDE: i16 = 0;
pub const ENIN_HORSE_ANIM_SITTING_FORWARD: i16 = 1;
pub const ENIN_HORSE_ANIM_REARING: i16 = 2;
pub const ENIN_HORSE_ANIM_LASHING_FAST: i16 = 3;
pub const ENIN_HORSE_ANIM_LASHING_SLOW: i16 = 4;
pub const ENIN_HORSE_ANIM_SITTING_UPRIGHT: i16 = 6;
pub const ENIN_HORSE_ANIM_WHOOPING: i16 = 7;

/// `sEponaAnimHeaders`, `sHniAnimHeaders`.
pub const EPONA_ANIMATIONS: [&str; 9] =
    ["gEponaIdleAnim", "gEponaWhinnyAnim", "gEponaRefuseAnim", "gEponaRearingAnim", "gEponaWalkingAnim", "gEponaTrottingAnim", "gEponaGallopingAnim", "gEponaJumpingAnim", "gEponaJumpingHighAnim"];
pub const HNI_ANIMATIONS: [&str; 9] = [
    "gHorseIngoIdleAnim",
    "gHorseIngoWhinnyAnim",
    "gHorseIngoRefuseAnim",
    "gHorseIngoRearingAnim",
    "gHorseIngoWalkingAnim",
    "gHorseIngoTrottingAnim",
    "gHorseIngoGallopingAnim",
    "gHorseIngoJumpingAnim",
    "gHorseIngoJumpingHighAnim",
];
/// `sSkeletonHeaders`.
const SKELETONS: [(&str, &str); 2] = [(OBJECT, "gEponaSkel"), (OBJECT_HNI, "gHorseIngoSkel")];

/// `sPlaybackSpeeds`.
const PLAYBACK_SPEEDS: [f32; 9] = [2.0 / 3.0, 2.0 / 3.0, 1.0, 1.0, 1.0, 1.0, 1.0, 2.0 / 3.0, 2.0 / 3.0];

/// `EnHorse_OverrideLimbDraw`'s `eyeTextures` and `eyeBlinkIndices` (Epona's).
pub const EYE_TEXTURES: [&str; 3] = ["gEponaEyeOpenTex", "gEponaEyeHalfTex", "gEponaEyeClosedTex"];
pub const EYE_BLINK_INDICES: [u8; 4] = [0, 1, 2, 1];
/// Segment 8: the eyes' texture (`gSPSegment(POLY_OPA_DISP++, 0x08, ..)` for limb 13).
const SEG_EYES: u32 = 0x0800_0000;

/// The two horses' meshes: their skins as `func_800A6360` draws them (Epona's eyes swapped per
/// draw).
pub const BAKE_EPONA: &str = "En_Horse/epona";
pub const BAKE_HNI: &str = "En_Horse/hni";

/// The meshes `EnHorse_Draw` draws.
pub fn bakes() -> Vec<MeshBake> {
    [(BAKE_EPONA, SKELETONS[0]), (BAKE_HNI, SKELETONS[1])]
        .into_iter()
        .map(|(name, (file, symbol))| MeshBake { name: name.into(), object: file.into(), segments: Vec::new(), prelude: Vec::new(), body: BakeBody::Skin { file: file.into(), symbol: symbol.into() } })
        .collect()
}

/// `sCylinderInit1`: the front, AT against enemies (`AT_TYPE_PLAYER`, `DMG_` 0x400, 4) and OC.
const CYLINDER_INIT1: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_TYPE_PLAYER, ac_flags: AC_NONE, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_1 | OC2_UNK1, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0x0000_0400, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x04 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: HIT_BACKLASH_NONE, defense: 0x00 },
        at_elem_flags: ATELEM_ON | ATELEM_SFX_NONE,
        ac_elem_flags: ACELEM_NONE,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 20, height: 70, y_shift: 0, pos: [0; 3] },
};

/// `sCylinderInit2`: the back, OC.
const CYLINDER_INIT2: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_NONE, ac_flags: AC_NONE, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_1 | OC2_UNK1, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0xFFCF_FFFF, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x00 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: HIT_BACKLASH_NONE, defense: 0x00 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_NONE,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 20, height: 70, y_shift: 0, pos: [0; 3] },
};

/// `sJntSphElementsInit`: the head, limb 13, radius 20, scale 100.
const JNT_SPH_ELEMENTS: [ColliderJntSphElementInit; 1] = [ColliderJntSphElementInit {
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0x0001_F824, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_ON | ACELEM_NO_AT_INFO | ACELEM_NO_DAMAGE | ACELEM_NO_SWORD_SFX | ACELEM_NO_HITMARK,
        oc_elem_flags: OCELEM_ON,
    },
    limb: 13,
    model_sphere: Sphere16 { center: [0, 0, 0], radius: 20 },
    scale: 100,
}];

/// `sJntSphInit`.
const JNT_SPH_INIT: ColliderInit = ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_NONE, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_1 | OC2_UNK1, shape: COLSHAPE_JNTSPH };

/// `D_80A65F38`.
const COL_CHK_INFO_INIT: CollisionCheckInfoInit = CollisionCheckInfoInit { health: 10, cyl_radius: 35, cyl_height: 100, mass: MASS_HEAVY };

/// `sHorseSpawns`: where Epona's Song brings her from, by scene (the nearest out of view).
#[rustfmt::skip]
pub const HORSE_SPAWNS: &[(u16, [i16; 3], i16)] = &[
    (SCENE_HYRULE_FIELD, [16, 0, 1341], 0), (SCENE_HYRULE_FIELD, [-1297, 0, 1459], 0), (SCENE_HYRULE_FIELD, [-5416, -300, 1296], 0),
    (SCENE_HYRULE_FIELD, [-4667, -300, 3620], 0), (SCENE_HYRULE_FIELD, [-3837, 81, 5537], 0), (SCENE_HYRULE_FIELD, [-5093, -226, 6661], 0),
    (SCENE_HYRULE_FIELD, [-6588, -79, 5053], 0), (SCENE_HYRULE_FIELD, [-7072, -500, 7538], 0), (SCENE_HYRULE_FIELD, [-6139, -500, 8910], 0),
    (SCENE_HYRULE_FIELD, [-8497, -300, 7802], 0), (SCENE_HYRULE_FIELD, [-5481, -499, 12127], 0), (SCENE_HYRULE_FIELD, [-4808, -700, 13583], 0),
    (SCENE_HYRULE_FIELD, [-3416, -490, 12092], 0), (SCENE_HYRULE_FIELD, [-2915, 100, 8339], 0), (SCENE_HYRULE_FIELD, [-2277, -500, 13247], 0),
    (SCENE_HYRULE_FIELD, [-1026, -500, 12101], 0), (SCENE_HYRULE_FIELD, [1427, -500, 13341], 0), (SCENE_HYRULE_FIELD, [-200, -486, 10205], 0),
    (SCENE_HYRULE_FIELD, [-1469, 100, 7496], 0), (SCENE_HYRULE_FIELD, [-995, 168, 5652], 0), (SCENE_HYRULE_FIELD, [1938, 89, 6232], 0),
    (SCENE_HYRULE_FIELD, [1387, -105, 9206], 0), (SCENE_HYRULE_FIELD, [1571, -223, 7701], 0), (SCENE_HYRULE_FIELD, [3893, -121, 7068], 0),
    (SCENE_HYRULE_FIELD, [3179, 373, 5221], 0), (SCENE_HYRULE_FIELD, [4678, -20, 3869], 0), (SCENE_HYRULE_FIELD, [3460, 246, 4207], 0),
    (SCENE_HYRULE_FIELD, [3686, 128, 2366], 0), (SCENE_HYRULE_FIELD, [1791, 18, 152], 0), (SCENE_HYRULE_FIELD, [3667, -16, 1399], 0),
    (SCENE_HYRULE_FIELD, [1827, -15, 983], 0), (SCENE_HYRULE_FIELD, [1574, 399, 4318], 0), (SCENE_HYRULE_FIELD, [716, 95, 3391], 0),
    (SCENE_HYRULE_FIELD, [-1189, -41, 4739], 0), (SCENE_HYRULE_FIELD, [-1976, -171, 4172], 0), (SCENE_HYRULE_FIELD, [1314, 391, 5665], 0),
    (SCENE_HYRULE_FIELD, [112, 0, 1959], 0), (SCENE_HYRULE_FIELD, [-3011, -111, 9397], 0), (SCENE_HYRULE_FIELD, [-5674, -270, 8585], 0),
    (SCENE_HYRULE_FIELD, [-8861, -300, 7836], 0), (SCENE_HYRULE_FIELD, [-6056, -500, 7810], 0), (SCENE_HYRULE_FIELD, [-7306, -500, 5994], 0),
    (SCENE_HYRULE_FIELD, [-7305, -500, 7605], 0), (SCENE_HYRULE_FIELD, [-7439, -300, 7600], 0), (SCENE_HYRULE_FIELD, [-7464, -300, 6268], 0),
    (SCENE_HYRULE_FIELD, [-8080, -300, 7553], 0), (SCENE_HYRULE_FIELD, [-8091, -300, 7349], 0), (SCENE_HYRULE_FIELD, [-8785, -300, 7383], 0),
    (SCENE_HYRULE_FIELD, [-8745, -300, 7508], 0), (SCENE_HYRULE_FIELD, [-8777, -300, 7788], 0), (SCENE_HYRULE_FIELD, [-8048, -299, 7738], 0),
    (SCENE_HYRULE_FIELD, [-7341, -184, 7730], 0), (SCENE_HYRULE_FIELD, [-6410, -288, 7824], 0), (SCENE_HYRULE_FIELD, [-6326, -290, 8205], 0),
    (SCENE_HYRULE_FIELD, [-6546, -292, 8400], 0), (SCENE_HYRULE_FIELD, [-7533, -180, 8459], 0), (SCENE_HYRULE_FIELD, [-8024, -295, 7903], 0),
    (SCENE_HYRULE_FIELD, [-8078, -308, 7994], 0), (SCENE_HYRULE_FIELD, [-9425, -287, 7696], 0), (SCENE_HYRULE_FIELD, [-9322, -292, 7577], 0),
    (SCENE_HYRULE_FIELD, [-9602, -199, 7160], 0), (SCENE_HYRULE_FIELD, [-9307, -300, 7758], 0), (SCENE_HYRULE_FIELD, [-9230, -300, 7642], 0),
    (SCENE_HYRULE_FIELD, [-7556, -499, 8695], 0), (SCENE_HYRULE_FIELD, [-6438, -500, 8606], 0), (SCENE_HYRULE_FIELD, [-6078, -500, 8258], 0),
    (SCENE_HYRULE_FIELD, [-6233, -500, 7613], 0), (SCENE_HYRULE_FIELD, [-5035, -205, 7814], 0), (SCENE_HYRULE_FIELD, [-5971, -500, 8501], 0),
    (SCENE_HYRULE_FIELD, [-5724, -498, 10123], 0), (SCENE_HYRULE_FIELD, [-5094, -392, 11106], 0), (SCENE_HYRULE_FIELD, [-5105, -393, 11312], 0),
    (SCENE_HYRULE_FIELD, [-4477, -314, 11132], 0), (SCENE_HYRULE_FIELD, [-3867, -380, 11419], 0), (SCENE_HYRULE_FIELD, [-2952, -500, 11944], 0),
    (SCENE_HYRULE_FIELD, [-2871, -488, 11743], 0), (SCENE_HYRULE_FIELD, [-3829, -372, 11327], 0), (SCENE_HYRULE_FIELD, [-4439, -293, 10989], 0),
    (SCENE_HYRULE_FIELD, [-5014, -381, 11086], 0), (SCENE_HYRULE_FIELD, [-5113, -188, 10968], 0), (SCENE_HYRULE_FIELD, [-5269, -188, 11156], 0),
    (SCENE_HYRULE_FIELD, [-5596, -178, 9972], 0), (SCENE_HYRULE_FIELD, [-5801, -288, 8518], 0), (SCENE_HYRULE_FIELD, [-4910, -178, 7931], 0),
    (SCENE_HYRULE_FIELD, [-3586, 73, 8140], 0), (SCENE_HYRULE_FIELD, [-4487, -106, 9362], 0), (SCENE_HYRULE_FIELD, [-4339, -112, 6412], 0),
    (SCENE_HYRULE_FIELD, [-3417, 105, 8194], 0), (SCENE_HYRULE_FIELD, [-4505, -20, 6608], 0), (SCENE_HYRULE_FIELD, [-5038, -199, 6603], 0),
    (SCENE_HYRULE_FIELD, [-4481, 1, 6448], 0), (SCENE_HYRULE_FIELD, [-5032, -168, 6418], 0), (SCENE_HYRULE_FIELD, [-5256, -700, 14329], 0),
    (SCENE_HYRULE_FIELD, [-5749, -820, 15380], 0), (SCENE_HYRULE_FIELD, [-3122, -700, 13608], 0), (SCENE_HYRULE_FIELD, [-3758, -525, 13228], 0),
    (SCENE_HYRULE_FIELD, [-3670, -500, 13123], 0), (SCENE_HYRULE_FIELD, [-2924, -500, 13526], 0), (SCENE_HYRULE_FIELD, [1389, -115, 9370], 0),
    (SCENE_HYRULE_FIELD, [548, -116, 8889], 0), (SCENE_HYRULE_FIELD, [-106, -107, 8530], 0), (SCENE_HYRULE_FIELD, [-1608, 85, 7646], 0),
    (SCENE_HYRULE_FIELD, [-5300, -700, 13772], 0), (SCENE_HYRULE_FIELD, [-5114, -700, 13400], 0), (SCENE_HYRULE_FIELD, [-4561, -700, 13700], 0),
    (SCENE_HYRULE_FIELD, [-4762, -700, 14084], 0), (SCENE_HYRULE_FIELD, [-2954, 100, 8216], 0), (SCENE_HYRULE_FIELD, [1460, -104, 9246], 0),
    (SCENE_HYRULE_FIELD, [629, -105, 8791], 0), (SCENE_HYRULE_FIELD, [-10, -90, 8419], 0), (SCENE_HYRULE_FIELD, [-1462, 100, 7504], 0),
    (SCENE_HYRULE_FIELD, [-3018, -500, 12493], 0), (SCENE_HYRULE_FIELD, [-2994, -311, 10331], 0), (SCENE_HYRULE_FIELD, [-4006, -700, 14152], 0),
    (SCENE_HYRULE_FIELD, [-4341, -500, 12743], 0), (SCENE_HYRULE_FIELD, [-5879, -497, 6799], 0), (SCENE_HYRULE_FIELD, [22, -473, 10103], 0),
    (SCENE_HYRULE_FIELD, [-1389, -192, 8874], 0), (SCENE_HYRULE_FIELD, [-4, 92, 6866], 0), (SCENE_HYRULE_FIELD, [483, 104, 6637], 0),
    (SCENE_HYRULE_FIELD, [1580, 183, 5987], 0), (SCENE_HYRULE_FIELD, [1548, 308, 5077], 0), (SCENE_HYRULE_FIELD, [1511, 399, 4267], 0),
    (SCENE_HYRULE_FIELD, [1358, 385, 4271], 0), (SCENE_HYRULE_FIELD, [1379, 395, 5063], 0), (SCENE_HYRULE_FIELD, [1360, 394, 5870], 0),
    (SCENE_HYRULE_FIELD, [813, 283, 6194], 0), (SCENE_HYRULE_FIELD, [-57, 101, 6743], 0), (SCENE_HYRULE_FIELD, [91, 325, 5143], 0),
    (SCENE_HYRULE_FIELD, [1425, -214, 7659], 0), (SCENE_HYRULE_FIELD, [3487, -19, 880], 0), (SCENE_HYRULE_FIELD, [2933, 152, 2094], 0),
    (SCENE_HYRULE_FIELD, [2888, -145, 6862], 0), (SCENE_HYRULE_FIELD, [1511, 67, 6471], 0), (SCENE_HYRULE_FIELD, [4051, -47, 1722], 0),
    (SCENE_HYRULE_FIELD, [-7335, -500, 8627], 0), (SCENE_HYRULE_FIELD, [-7728, -462, 8498], 0), (SCENE_HYRULE_FIELD, [-7791, -446, 8832], 0),
    (SCENE_HYRULE_FIELD, [-2915, -435, 11334], 0), (SCENE_HYRULE_FIELD, [165, -278, 3352], 0),
    (SCENE_LAKE_HYLIA, [-2109, -882, 1724], 0), (SCENE_LAKE_HYLIA, [-328, -1238, 2705], 0), (SCENE_LAKE_HYLIA, [-3092, -1033, 3527], 0),
    (SCENE_GERUDO_VALLEY, [2687, -269, 753], 0), (SCENE_GERUDO_VALLEY, [2066, -132, 317], 0), (SCENE_GERUDO_VALLEY, [523, -8, 635], 0),
    (SCENE_GERUDO_VALLEY, [558, 36, -323], 0), (SCENE_GERUDO_VALLEY, [615, 51, -839], 0), (SCENE_GERUDO_VALLEY, [-614, 32, 29], 0),
    (SCENE_GERUDO_VALLEY, [-639, -3, 553], 0), (SCENE_GERUDO_VALLEY, [-540, 10, -889], 0), (SCENE_GERUDO_VALLEY, [-1666, 58, 378], 0),
    (SCENE_GERUDO_VALLEY, [-3044, 210, -648], 0),
    (SCENE_GERUDOS_FORTRESS, [-678, 21, -623], 0), (SCENE_GERUDOS_FORTRESS, [149, 333, -2499], 0), (SCENE_GERUDOS_FORTRESS, [499, 581, -547], 0),
    (SCENE_GERUDOS_FORTRESS, [3187, 1413, -3775], 0), (SCENE_GERUDOS_FORTRESS, [3198, 1413, 307], 0), (SCENE_GERUDOS_FORTRESS, [3380, 1413, -1200], 0),
    (SCENE_GERUDOS_FORTRESS, [-966, 1, -56], 0), (SCENE_GERUDOS_FORTRESS, [-966, 24, -761], 0), (SCENE_GERUDOS_FORTRESS, [-694, 174, -2820], 0),
    (SCENE_LON_LON_RANCH, [1039, 0, 2051], 0), (SCENE_LON_LON_RANCH, [-1443, 0, 1429], 0),
    // Hardcoded to always load in Lon Lon.
    (SCENE_LON_LON_RANCH, [856, 0, -918], 0),
    (SCENE_LON_LON_RANCH, [882, 0, -2256], 0),
    // Hardcoded to always load in Lon Lon.
    (SCENE_LON_LON_RANCH, [-1003, 0, -755], 0),
    (SCENE_LON_LON_RANCH, [-2254, 0, -629], 0), (SCENE_LON_LON_RANCH, [907, 0, -2336], 0),
];

/// `BridgeJumpPoint`: z and x ranges, the x offset, the angle and its range, where it lands.
struct BridgeJumpPoint {
    z_min: i16,
    z_max: i16,
    x_min: i16,
    x_max: i16,
    x_offset: i16,
    angle: i16,
    angle_range: i16,
    pos: [i16; 3],
}

/// `sBridgeJumps`: Gerudo Valley's broken bridge, both ways.
const BRIDGE_JUMPS: [BridgeJumpPoint; 2] = [
    BridgeJumpPoint { z_min: -195, z_max: -40, x_min: 225, x_max: 120, x_offset: 360, angle: -0x4000, angle_range: 0x7D0, pos: [-270, -52, -117] },
    BridgeJumpPoint { z_min: -195, z_max: -40, x_min: -240, x_max: -120, x_offset: -360, angle: 0x4000, angle_range: 0x7D0, pos: [270, -52, -117] },
];

/// `RaceWaypoint`: a place, the speed there and the line's angle that passes it.
#[derive(Debug, Clone, Copy)]
struct RaceWaypoint {
    x: i16,
    y: i16,
    z: i16,
    speed_xz: i16,
    angle: i16,
}

/// `sIngoRaceWaypoints`.
const INGO_RACE_WAYPOINTS: [RaceWaypoint; 8] = [
    RaceWaypoint { x: 1056, y: 1, z: -1540, speed_xz: 11, angle: 0x2A8D },
    RaceWaypoint { x: 1593, y: 1, z: -985, speed_xz: 10, angle: 0xFC27u16 as i16 },
    RaceWaypoint { x: 1645, y: 1, z: -221, speed_xz: 11, angle: 0xE891u16 as i16 },
    RaceWaypoint { x: 985, y: 1, z: 403, speed_xz: 10, angle: 0xBB9Cu16 as i16 },
    RaceWaypoint { x: -1023, y: 1, z: 354, speed_xz: 11, angle: 0xA37Du16 as i16 },
    RaceWaypoint { x: -1679, y: 1, z: -213, speed_xz: 10, angle: 0x889Cu16 as i16 },
    RaceWaypoint { x: -1552, y: 1, z: -1008, speed_xz: 11, angle: 0x638D },
    RaceWaypoint { x: -947, y: -1, z: -1604, speed_xz: 10, angle: 0x4002 },
];

/// `sAnimSoundFrames`: the walk's hooves.
const ANIM_SOUND_FRAMES: [i32; 2] = [0, 16];
/// `sResetNoInput`, by `params`.
const RESET_NO_INPUT: [u8; 12] = [0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 0];
/// `sIdleAnimIds`.
const IDLE_ANIM_IDS: [i32; 6] = [ENHORSE_ANIM_WHINNEY, ENHORSE_ANIM_REARING, ENHORSE_ANIM_IDLE, ENHORSE_ANIM_REARING, ENHORSE_ANIM_WHINNEY, ENHORSE_ANIM_IDLE];
/// `sIngoAnimations`, by the horse's animation.
const INGO_ANIMATIONS: [i16; 10] = [
    ENIN_HORSE_ANIM_WHOOPING,
    ENIN_HORSE_ANIM_SITTING_UPRIGHT,
    ENIN_HORSE_ANIM_REARING,
    ENIN_HORSE_ANIM_REARING,
    ENIN_HORSE_ANIM_SITTING_FORWARD,
    ENIN_HORSE_ANIM_SITTING_FORWARD,
    ENIN_HORSE_ANIM_SLOW_RIDE,
    ENIN_HORSE_ANIM_SLOW_RIDE,
    ENIN_HORSE_ANIM_SLOW_RIDE,
    ENIN_HORSE_ANIM_SLOW_RIDE,
];

/// `sCsActionTable`: the player cues (`PLAYER_CUEID_*`) a cutscene horse follows, and its function.
const CS_ACTION_TABLE: [(u16, i32); 5] = [(36, 1), (37, 2), (38, 3), (64, 4), (65, 5)];

/// `postDrawFunc`: what the draw starts after the post-draw.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostDraw {
    /// `EnHorse_LowJumpInit`.
    LowJumpInit,
    /// `EnHorse_HighJumpInit`.
    HighJumpInit,
    /// `EnHorse_BridgeJumpInit`.
    BridgeJumpInit,
}

/// `PLAYER_STATE1_*` the horse reads (`player.h`).
const PLAYER_STATE1_0: u32 = 1 << 0;
const PLAYER_STATE1_20: u32 = 1 << 20;

// The render state's extras.
mod rs {
    /// `switches`: the eye texture, then 1 while drawn without the root's translation (jumping).
    pub const EYES: usize = 0;
    pub const NO_TRANSLATION: usize = 1;
}

pub struct EnHorse {
    pub actor: Actor,
    pub action: i32,
    pub no_input_timer: i32,
    pub no_input_timer_max: i32,
    /// `type`: `HORSE_EPONA` or `HORSE_HNI`.
    pub ty: i32,
    /// `skin.skelAnime`.
    pub skel: SkelAnimeStd,
    pub skeleton: Arc<SkinSkeleton>,
    pub state_flags: u32,
    pub last_pos: Vec3,
    pub last_yaw: i16,
    pub cur_race_waypoint: i32,
    pub boost_speed: i32,
    pub player_controlled: bool,
    pub animation_idx: i32,
    /// `curFrame`: the animation's frame as the last update left it.
    pub cur_frame: f32,
    pub sound_timer: i32,
    /// `unk_21C`: where her voice plays (`unk_228`, the head, when it plays).
    pub unk_21c: Vec3,
    /// `unk_228`: the head's projected position (the post-draw's).
    pub unk_228: Vec3,
    pub unk_234: i32,
    pub num_boosts: u8,
    pub boost_regen_time: i32,
    pub boost_timer: i32,
    pub post_draw_func: Option<PostDraw>,
    /// `yFront`, `yBack`: the floor under the front and back feet.
    pub y_front: f32,
    pub y_back: f32,
    pub follow_timer: i16,
    pub prev_action: i32,
    /// `riderPos`: where Link rides, from the horse's place (the post-draw's limb 30).
    pub rider_pos: Vec3,
    pub cur_stick: Vec2,
    pub last_stick: Vec2,
    pub jump_start_y: f32,
    pub collider_cylinder1: ColliderCylinder,
    pub collider_cylinder2: ColliderCylinder,
    pub collider_jnt_sph: ColliderJntSph,
    pub player_dir: u32,
    pub angle_to_player: i16,
    pub follow_player_turn_speed: i16,
    pub blink_timer: u8,
    pub wait_timer: i16,
    pub cutscene_action: i32,
    pub cutscene_flags: u16,
    pub in_race: bool,
    /// `rider`: Ingo on his horse (`En_In` 1).
    pub rider: Option<ActorHandle>,
    pub ingo_race_flags: u16,
    pub ingo_horse_max_speed: f32,
    /// `unk_39C`, `hbaStarted`, `hbaFlags`, `hbaTimer`: horseback archery's (Phase 18).
    pub unk_39c: i32,
    pub hba_started: i32,
    pub hba_flags: i32,
    pub hba_timer: i32,
    pub bridge_jump_idx: u8,
    pub bridge_jump_start: Vec3,
    pub bridge_jump_timer: i32,
    pub bridge_jump_y_vel: f32,
    pub bridge_jump_rel_angle: i16,
    pub dust_flags: u16,
    pub front_right_hoof: Vec3,
    pub front_left_hoof: Vec3,
    pub back_right_hoof: Vec3,
    pub back_left_hoof: Vec3,
    anims: Vec<Anim>,
    eyes: Vec<DrawImage>,
    logged_hba: bool,
}

/// `EnHorse_Vec3fOffset`.
fn vec3f_offset(src: Vec3, yaw: i16, dist: f32, height: f32) -> Vec3 {
    Vec3::new(src.x + sin_s(yaw) * dist, src.y + height, src.z + cos_s(yaw) * dist)
}

/// `Math3D_RotateXZPlane`: the plane through the point at the angle (`a`, `c` times 32767).
fn rotate_xz_plane(p: Vec3, angle: i16) -> (f32, f32, f32) {
    let a = sin_s(angle) * 32767.0;
    let c = cos_s(angle) * 32767.0;
    let d = -((a * p.x) + (c * p.z));
    (a, c, d)
}

/// `Math3D_PointDistSqToLine2D`'s `lineLenSq`: the squared distance from (x0, y0) to the line
/// through (x1, y1) and (x2, y2).
fn point_dist_sq_to_line_2d(x0: f32, y0: f32, x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    let x_diff = x2 - x1;
    let y_diff = y2 - y1;
    let dist_sq = x_diff * x_diff + y_diff * y_diff;
    if dist_sq.abs() < 0.008 {
        return 0.0;
    }
    let r = (((x0 - x1) * x_diff) + (y0 - y1) * y_diff) / dist_sq;
    let px = (x_diff * r) + x1;
    let py = (y_diff * r) + y1;
    (px - x0) * (px - x0) + (py - y0) * (py - y0)
}

/// `RAD_TO_BINANG(Math_FAtan2F(y, x))`.
fn atan2_binang(y: f32, x: f32) -> i16 {
    rad_to_binang_trunc(f_atan2f(y, x))
}

/// An f32 into an s16 as the C converts it (truncated, then the low 16 bits).
fn f2s(v: f32) -> i16 {
    v as i32 as i16
}

/// Whether the actor with `bg` is a `Bg_Umajump`: `DynaPoly_GetActor(&play->colCtx, bgId)`'s
/// `actor.id`.
fn dyna_actor_id(play: &PlayState, bg: u16) -> Option<i16> {
    oot_game::actor_ctx::dyna_poly_get_actor(&play.actors, &play.col, bg).and_then(|h| play.actors.actor(h)).map(|a| a.id)
}

impl EnHorse {
    /// `EnHorse_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        play.save.r_exited_scene_riding_horse = false;
        // sInitChain: cullingVolumeScale 600, cullingVolumeDownward 300.
        actor.culling_volume_scale = 600.0;
        actor.culling_volume_downward = 300.0;
        // R_EPONAS_SONG_PLAYED = false.
        play.msg_ctx.r_eponas_song_played = false;
        let mut rider_pos = actor.world_pos;
        rider_pos.y += 70.0;
        // DREG(4): 70 (the obstacle's drop beyond, `EnHorse_UpdateBgCheckInfo`).

        let ty;
        let boost_speed;
        if TYPE.get(actor.params) == 1 {
            actor.params = PTYPE.get(actor.params);
            ty = HORSE_HNI;
            if play.object_ctx.get_index(OBJECT_HNI_ID).is_none() {
                actor.kill();
                return oot_game::spawn::Placeholder::init(actor, play);
            }
            boost_speed = 12;
        } else {
            ty = HORSE_EPONA;
            boost_speed = 14;
        }
        let Some(assets) = play.assets.clone() else {
            actor.kill();
            return oot_game::spawn::Placeholder::init(actor, play);
        };
        let (file, skel_name) = SKELETONS[ty as usize];
        let names = if ty == HORSE_EPONA { &EPONA_ANIMATIONS } else { &HNI_ANIMATIONS };
        let loaded = (|| -> anyhow::Result<_> {
            let skeleton = assets.pack.skin_skeleton(file, skel_name)?;
            let anims = names.iter().map(|a| assets.animation(file, a)).collect::<anyhow::Result<Vec<_>>>()?;
            let mut eyes = Vec::new();
            if ty == HORSE_EPONA {
                for s in EYE_TEXTURES {
                    let t = assets.pack.texture(OBJECT, s)?;
                    eyes.push(DrawImage { width: t.width, height: t.height, rgba: t.rgba.clone().into() });
                }
            }
            Ok((skeleton, anims, eyes))
        })();
        let (skeleton, anims, eyes) = match loaded {
            Ok(x) => x,
            Err(e) => {
                log::error!("En_Horse: {e:#}");
                actor.kill();
                return oot_game::spawn::Placeholder::init(actor, play);
            }
        };

        // params was -1.
        if PTYPE.get(actor.params) == 0x7FFF {
            actor.params = HORSE_PTYPE_1;
        }
        let epona = play.save.get_event_chk_inf(EVENTCHKINF_EPONA_OBTAINED) || R_DEBUG_FORCE_EPONA_OBTAINED != 0;
        let mut state_flags = if play.scene_id == SCENE_LON_LON_BUILDINGS {
            ENHORSE_UNRIDEABLE
        } else if play.scene_id == SCENE_GERUDOS_FORTRESS && ty == HORSE_HNI {
            ENHORSE_FLAG_18 | ENHORSE_UNRIDEABLE
        } else if PTYPE.get(actor.params) == HORSE_PTYPE_INGO_SPAWNED_RIDING {
            ENHORSE_FLAG_19 | ENHORSE_CANT_JUMP | ENHORSE_UNRIDEABLE
        } else if PTYPE.get(actor.params) == HORSE_PTYPE_6 {
            let mut f = ENHORSE_FLAG_19 | ENHORSE_CANT_JUMP;
            if epona {
                f &= !ENHORSE_CANT_JUMP;
                f |= ENHORSE_FLAG_26;
            } else if play.save.get_event_inf(EVENTINF_INGO_RACE_SECOND_RACE) && ty == HORSE_HNI {
                f |= ENHORSE_FLAG_21 | ENHORSE_FLAG_20;
            }
            f
        } else if PTYPE.get(actor.params) == HORSE_PTYPE_1 {
            ENHORSE_FLAG_7
        } else {
            0
        };
        if play.scene_id == SCENE_LON_LON_RANCH && ingo_race_state(&play.save) == INGO_RACE_STATE_TRAPPED_WIN_EPONA && !epona {
            state_flags |= ENHORSE_FLAG_25;
        }

        actor.scale = Vec3::splat(0.01);
        actor.gravity = -3.5;
        actor.shape_init(0.0, oot_game::shadow::ShadowDraw::Horse, 20.0);
        actor.speed_xz = 0.0;
        actor.focus_pos = actor.world_pos + Vec3::Y * 70.0;
        // Skin_Init(.., sSkeletonHeaders[type], sAnimationHeaders[type][IDLE]): SkelAnime_InitSkin
        // plays it looped; then Animation_PlayOnce(sAnimationHeaders[type][animationIdx]).
        let mut skel = SkelAnimeStd::init_flex(skeleton.limbs.len(), Some(anims[ENHORSE_ANIM_IDLE as usize].clone()));
        skel.play_once(anims[ENHORSE_ANIM_IDLE as usize].clone());
        let mut this = EnHorse {
            actor,
            action: ENHORSE_ACT_IDLE,
            no_input_timer: 0,
            no_input_timer_max: 0,
            ty,
            skel,
            skeleton: Arc::new(skeleton),
            state_flags,
            last_pos: Vec3::ZERO,
            last_yaw: 0,
            cur_race_waypoint: 0,
            boost_speed,
            player_controlled: false,
            animation_idx: ENHORSE_ANIM_IDLE,
            cur_frame: 0.0,
            sound_timer: 0,
            unk_21c: Vec3::ZERO,
            unk_228: Vec3::ZERO,
            unk_234: 0,
            num_boosts: 6,
            boost_regen_time: 0,
            boost_timer: 0,
            post_draw_func: None,
            y_front: 0.0,
            y_back: 0.0,
            follow_timer: 0,
            prev_action: 0,
            rider_pos,
            cur_stick: Vec2::ZERO,
            last_stick: Vec2::ZERO,
            jump_start_y: 0.0,
            collider_cylinder1: ColliderCylinder::new(&CYLINDER_INIT1),
            collider_cylinder2: ColliderCylinder::new(&CYLINDER_INIT2),
            collider_jnt_sph: ColliderJntSph::new(&JNT_SPH_INIT, &JNT_SPH_ELEMENTS),
            player_dir: 0,
            angle_to_player: 0,
            follow_player_turn_speed: 0,
            blink_timer: 0,
            wait_timer: 0,
            cutscene_action: -1,
            cutscene_flags: 0,
            in_race: false,
            rider: None,
            ingo_race_flags: 0,
            ingo_horse_max_speed: 0.0,
            unk_39c: 0,
            hba_started: 0,
            hba_flags: 0,
            hba_timer: 0,
            bridge_jump_idx: 0,
            bridge_jump_start: Vec3::ZERO,
            bridge_jump_timer: 0,
            bridge_jump_y_vel: 0.0,
            bridge_jump_rel_angle: 0,
            dust_flags: 0,
            front_right_hoof: Vec3::ZERO,
            front_left_hoof: Vec3::ZERO,
            back_right_hoof: Vec3::ZERO,
            back_left_hoof: Vec3::ZERO,
            anims,
            eyes,
            logged_hba: false,
        };
        this.actor.col_chk_info.set_info(None, &COL_CHK_INFO_INIT);

        if play.scene_id == SCENE_LON_LON_RANCH && !play.is_cutscene_layer() {
            if this.ty == HORSE_HNI {
                if ROT_Z.get(this.actor.world_rot.z) == 0 || !play.save.is_day() {
                    this.actor.kill();
                    return Box::new(this);
                }
                if play.save.get_event_chk_inf(EVENTCHKINF_EPONA_OBTAINED) {
                    this.actor.kill();
                    return Box::new(this);
                }
                if ROT_Z.get(this.actor.world_rot.z) != 5 {
                    this.actor.kill();
                    return Box::new(this);
                }
            } else if !epona && !play.save.is_day() {
                this.actor.kill();
                return Box::new(this);
            }
        } else if play.scene_id == SCENE_STABLE {
            if play.save.is_day() || epona || !play.save.adult {
                this.actor.kill();
                return Box::new(this);
            }
            this.state_flags |= ENHORSE_UNRIDEABLE;
        }

        // (Skin_Init above.) EnHorse_ResetCutscene, _ResetRace, _ResetHorsebackArchery.
        this.cutscene_action = -1;
        this.cutscene_flags = 0;
        this.in_race = false;
        this.unk_39c = 0;
        this.hba_started = 0;
        this.hba_flags = 0;

        if PTYPE.get(this.actor.params) == HORSE_PTYPE_INACTIVE {
            this.init_inactive();
        } else if PTYPE.get(this.actor.params) == HORSE_PTYPE_INGO_SPAWNED_RIDING {
            this.init_ingo_horse();
            let p = this.actor.world_pos;
            let (rx, ry) = (this.actor.shape_rot.x, this.actor.shape_rot.y);
            match play.actor_spawn(crate::en_in::ACTOR_EN_IN, p, [rx, ry, 1], 1) {
                Ok(h) => this.rider = Some(h),
                Err(e) => log::error!("En_Horse: Ingo (En_In 1) didn't spawn ({e:?})"),
            }
            this.ingo_horse_max_speed = if !play.save.get_event_inf(EVENTINF_INGO_RACE_SECOND_RACE) { 12.07 } else { 12.625 };
        } else if PTYPE.get(this.actor.params) == HORSE_PTYPE_7 {
            this.init_cutscene();
        } else if PTYPE.get(this.actor.params) == HORSE_PTYPE_HORSEBACK_ARCHERY {
            // EnHorse_InitHorsebackArchery, Interface_InitHorsebackArchery: Phase 18.
            log::info!("En_Horse: horseback archery (HORSE_PTYPE_HORSEBACK_ARCHERY) isn't ported (Phase 18): it stands");
            this.hba_started = 0;
            this.sound_timer = 0;
            this.cur_race_waypoint = 0;
            this.hba_timer = 0;
            this.actor.speed_xz = 0.0;
            this.action = ENHORSE_ACT_HBA;
        } else if play.scene_id == SCENE_LON_LON_RANCH && !play.save.get_event_chk_inf(EVENTCHKINF_EPONA_OBTAINED) && R_DEBUG_FORCE_EPONA_OBTAINED == 0 {
            this.init_flee_player();
        } else if play.scene_id == SCENE_LON_LON_BUILDINGS || (play.scene_id == SCENE_GERUDOS_FORTRESS && this.ty == HORSE_HNI) {
            this.reset_idle_animation(play);
        } else {
            this.start_idle_ridable(play);
        }
        this.actor.home_rot.z = 0;
        this.actor.world_rot.z = 0;
        this.actor.shape_rot.z = 0;
        Box::new(this)
    }

    fn anim(&self, idx: i32) -> Anim {
        self.anims[idx as usize].clone()
    }

    /// `Animation_Change(&skin.skelAnime, sAnimationHeaders[type][animationIdx], speed, start,
    /// Animation_GetLastFrame(..), mode, morph)`.
    fn change(&mut self, speed: f32, start: f32, mode: u8, morph: f32) {
        let a = self.anim(self.animation_idx);
        let last = a.last_frame();
        self.skel.change(a, speed, start, last, mode, morph);
    }

    /// `Animation_PlayOnce(&skin.skelAnime, sAnimationHeaders[type][animationIdx])`.
    fn play_once(&mut self) {
        let a = self.anim(self.animation_idx);
        self.skel.play_once(a);
    }

    /// `Animation_PlayOnceSetSpeed`.
    fn play_once_set_speed(&mut self, speed: f32) {
        self.change(speed, 0.0, ANIMMODE_ONCE, 0.0);
    }

    /// The root joint's y (`jointTable->y`, the root's translation).
    fn root_y(&self) -> f32 {
        self.skel.joint_table.first().map(|j| j[1] as f32).unwrap_or(0.0)
    }

    /// `SFX_PLAY_AT_POS(&this->unk_21C, sfx)` after `unk_21C = unk_228`, if drawn last frame:
    /// her voice, at the head (here at the horse: BACKLOG #68).
    fn voice(&mut self, play: &mut PlayState, sfx: u16) {
        self.unk_21c = self.unk_228;
        if self.state_flags & ENHORSE_DRAW != 0 {
            audio_play_actor_sfx2(play, sfx);
        }
    }

    /// `EnHorse_PlayerCanMove`.
    fn player_can_move(&self, play: &PlayState) -> bool {
        let Some(p) = play.player.and_then(|h| play.actors.get(h)) else { return false };
        let pi = p.as_player();
        let s1 = pi.map(|p| p.state_flags1()).unwrap_or(0);
        let aiming = pi.is_some_and(|p| p.func_8002dd78());
        !(s1 & PLAYER_STATE1_0 != 0
            || aiming
            || s1 & PLAYER_STATE1_20 != 0
            || (self.state_flags & ENHORSE_FLAG_19 != 0 && !self.in_race)
            || self.action == ENHORSE_ACT_HBA
            || p.base().flags & ACTOR_FLAG_TALK != 0
            || play.cs_ctx.state != CS_STATE_IDLE)
    }

    fn player_pos(play: &PlayState) -> Vec3 {
        play.player.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos).unwrap_or(Vec3::ZERO)
    }

    /// `EnHorse_RotateToPoint` (`Horse_RotateToPoint`).
    fn rotate_to_point(&mut self, pos: Vec3, turn: i16) {
        horse_rotate_to_point(&mut self.actor, pos, turn);
    }

    /// `EnHorse_BgCheckBridgeJumpPoint`.
    fn bg_check_bridge_jump_point(&self, play: &PlayState) -> bool {
        if play.scene_id != SCENE_GERUDO_VALLEY || self.actor.speed_xz < 12.8 || carpenters_all_rescued(play) {
            return false;
        }
        for b in &BRIDGE_JUMPS {
            let mut x_min = b.x_min as f32;
            let mut x_max = (x_min + b.x_max as f32) + b.x_offset as f32;
            if x_max < x_min {
                std::mem::swap(&mut x_min, &mut x_max);
            }
            let p = self.actor.world_pos;
            if (b.z_min as f32) < p.z && p.z < b.z_max as f32 && x_min < p.x && p.x < x_max {
                let r = self.actor.world_rot.y as i32;
                if (b.angle as i32 - b.angle_range as i32) < r && r < (b.angle as i32 + b.angle_range as i32) {
                    return true;
                }
            }
        }
        false
    }

    /// `EnHorse_CheckBridgeJumps`: where the jump would start, `EnHorse_StartBridgeJump`'s script
    /// (Phase 18) is logged instead.
    fn check_bridge_jumps(&mut self, play: &PlayState) -> bool {
        if self.actor.speed_xz < 12.8 {
            return false;
        }
        for (i, b) in BRIDGE_JUMPS.iter().enumerate() {
            let mut x_min = b.x_min as f32;
            let mut x_max = b.x_max as f32 + x_min;
            if x_max < x_min {
                std::mem::swap(&mut x_min, &mut x_max);
            }
            let p = self.actor.world_pos;
            if (b.z_min as f32) < p.z && p.z < b.z_max as f32 && x_min < p.x && p.x < x_max {
                let r = self.actor.world_rot.y as i32;
                if (b.angle as i32 - b.angle_range as i32) < r && r < (b.angle as i32 + b.angle_range as i32) {
                    self.bridge_jump_idx = i as u8;
                    // EnHorse_StartBridgeJump: postDrawFunc = EnHorse_BridgeJumpInit and the
                    // valley's script (gGerudoValleyBridgeJump*Cs) started: Phase 18.
                    log::info!("En_Horse: Gerudo Valley's bridge jump {i} isn't started (its script is Phase 18's)");
                    let _ = play;
                    return true;
                }
            }
        }
        false
    }

    /// `EnHorse_UpdateIngoRaceInfo`.
    fn update_ingo_race_info(&mut self, play: &mut PlayState) {
        let waypoints = &INGO_RACE_WAYPOINTS;
        let wp = |i: i32| Vec3::new(waypoints[i as usize].x as f32, waypoints[i as usize].y as f32, waypoints[i as usize].z as f32);
        let cur = wp(self.cur_race_waypoint);
        let (px, pz, d) = rotate_xz_plane(cur, waypoints[self.cur_race_waypoint as usize].angle);
        if (self.actor.world_pos.x * px) + (pz * self.actor.world_pos.z) + d > 0.0 {
            self.cur_race_waypoint += 1;
            if self.cur_race_waypoint >= waypoints.len() as i32 {
                self.cur_race_waypoint = 0;
            }
        }
        let cur = wp(self.cur_race_waypoint);
        let mut prev = self.cur_race_waypoint - 1;
        if prev < 0 {
            prev = waypoints.len() as i32 - 1;
        }
        let prev_pos = wp(prev);
        let dist_sq = point_dist_sq_to_line_2d(self.actor.world_pos.x, self.actor.world_pos.z, prev_pos.x, prev_pos.z, cur.x, cur.z);
        self.rotate_to_point(cur, 400);

        if dist_sq < 300.0 * 300.0 {
            let hit = self.collider_jnt_sph.elements[0].info.oc_elem_flags & OCELEM_HIT != 0;
            let toward = self.actor.yaw_towards_player.wrapping_sub(self.actor.world_rot.y);
            if self.actor.xz_dist_to_player < 130.0 || hit {
                if sin_s(toward) > 0.0 {
                    self.actor.world_rot.y = self.actor.world_rot.y.wrapping_sub(280);
                } else {
                    self.actor.world_rot.y = self.actor.world_rot.y.wrapping_add(280);
                }
            } else if self.actor.xz_dist_to_player < 300.0 {
                if sin_s(toward) > 0.0 {
                    self.actor.world_rot.y = self.actor.world_rot.y.wrapping_add(280);
                } else {
                    self.actor.world_rot.y = self.actor.world_rot.y.wrapping_sub(280);
                }
            }
            self.actor.shape_rot.y = self.actor.world_rot.y;
        }

        let pp = Self::player_pos(play);
        let sp50 = ((pp.x - self.actor.world_pos.x).powi(2) + (pp.z - self.actor.world_pos.z).powi(2)).sqrt();
        let rel = yaw_to(self.actor.world_pos, pp).wrapping_sub(self.actor.world_rot.y);
        if sp50 <= 200.0 || (sin_s(rel).abs() < 0.8 && cos_s(rel) > 0.0) {
            if self.actor.speed_xz < self.ingo_horse_max_speed {
                self.actor.speed_xz += 0.47;
            } else {
                self.actor.speed_xz -= 0.47;
            }
            self.ingo_race_flags |= INGO_FLAG_LASHING;
            return;
        }
        if self.actor.speed_xz < waypoints[self.cur_race_waypoint as usize].speed_xz as f32 {
            self.actor.speed_xz += 0.4;
        } else {
            self.actor.speed_xz -= 0.4;
        }
        self.ingo_race_flags &= !INGO_FLAG_LASHING;
    }

    /// `EnHorse_PlayWalkingSfx`.
    fn play_walking_sfx(&mut self, play: &mut PlayState) {
        if (ANIM_SOUND_FRAMES[self.sound_timer as usize] as f32) < self.cur_frame {
            if self.sound_timer == 0 && (ANIM_SOUND_FRAMES[1] as f32) < self.cur_frame {
                return;
            }
            audio_play_actor_sfx2(play, NA_SE_EV_HORSE_WALK);
            self.sound_timer += 1;
            if self.sound_timer >= ANIM_SOUND_FRAMES.len() as i32 {
                self.sound_timer = 0;
            }
        }
    }

    /// `EnHorse_PlayTrottingSfx`, `EnHorse_PlayGallopingSfx`.
    fn play_run_sfx(play: &mut PlayState) {
        audio_play_actor_sfx2(play, NA_SE_EV_HORSE_RUN);
    }

    /// `EnHorse_SlopeSpeedMultiplier`.
    fn slope_speed_multiplier(&self) -> f32 {
        if cos_s(self.actor.shape_rot.x) < 0.939262 && sin_s(self.actor.shape_rot.x) < 0.0 { 0.7 } else { 1.0 }
    }

    /// `func_80A5BBBC`: the point is in view (in the culling volume, or the eye within 100).
    fn func_80a5bbbc(&self, play: &PlayState, pos: Vec3) -> bool {
        let c = play.view_proj * pos.extend(1.0);
        let w = c.w;
        if w.abs() < 0.008 {
            return false;
        }
        let eye_dist = pos.distance(play.view.eye);
        oot_game::play::actor_culling_volume_test(&self.actor, c.truncate(), w) || eye_dist < 100.0
    }

    /// `EnHorse_IdleAnimSounds`.
    fn idle_anim_sounds(&mut self, play: &mut PlayState) {
        if self.animation_idx == ENHORSE_ANIM_IDLE
            && ((self.cur_frame > 35.0 && self.ty == HORSE_EPONA) || (self.cur_frame > 28.0 && self.ty == HORSE_HNI))
            && self.state_flags & ENHORSE_SANDDUST_SOUND == 0
        {
            self.state_flags |= ENHORSE_SANDDUST_SOUND;
            audio_play_actor_sfx2(play, NA_SE_EV_HORSE_SANDDUST);
        } else if self.animation_idx == ENHORSE_ANIM_REARING && self.cur_frame > 25.0 && self.state_flags & ENHORSE_LAND2_SOUND == 0 {
            self.state_flags |= ENHORSE_LAND2_SOUND;
            audio_play_actor_sfx2(play, NA_SE_EV_HORSE_LAND2);
        }
    }

    /// `EnHorse_Spawn`: to the nearest of the scene's `sHorseSpawns` out of view (on the ranch only
    /// its two inside, until Epona is had), facing Link.
    fn spawn(&mut self, play: &mut PlayState) -> bool {
        let mut min_dist = 1e38_f32;
        let mut spawned = false;
        let pp = Self::player_pos(play);
        let epona = play.save.get_event_chk_inf(EVENTCHKINF_EPONA_OBTAINED);
        for &(scene, pos, angle) in HORSE_SPAWNS.iter() {
            if scene != play.scene_id {
                continue;
            }
            // @bug (game): the same flag checked twice.
            let inside = (pos == [856, 0, -918]) || (pos == [-1003, 0, -755]);
            if play.scene_id != SCENE_LON_LON_RANCH || (epona && (ingo_race_state(&play.save) != INGO_RACE_STATE_TRAPPED_WIN_EPONA || epona)) || inside {
                let spawn_pos = Vec3::new(pos[0] as f32, pos[1] as f32, pos[2] as f32);
                let dist = pp.distance(spawn_pos);
                if min_dist < dist || self.func_80a5bbbc(play, spawn_pos) {
                    continue;
                }
                min_dist = dist;
                self.actor.world_pos = spawn_pos;
                self.actor.prev_pos = spawn_pos;
                self.actor.world_rot.y = angle;
                self.actor.shape_rot.y = yaw_to(self.actor.world_pos, pp);
                spawned = true;
                // (OOT_VERSION >= PAL_1_0) its projected position.
                let c = play.view_proj * self.actor.world_pos.extend(1.0);
                self.actor.projected_pos = c.truncate();
                self.actor.projected_w = c.w;
            }
        }
        spawned
    }

    /// `EnHorse_RotateToPlayer`.
    fn rotate_to_player(&mut self, play: &PlayState) {
        let pp = Self::player_pos(play);
        self.rotate_to_point(pp, 400);
        if self.state_flags & ENHORSE_OBSTACLE != 0 {
            self.actor.world_rot.y = f2s(self.actor.world_rot.y as f32 + 800.0);
        }
        self.actor.shape_rot.y = self.actor.world_rot.y;
    }

    /// `EnHorse_Freeze`.
    fn freeze(&mut self) {
        if self.action != ENHORSE_ACT_CS_UPDATE && self.action != ENHORSE_ACT_HBA {
            let p = PTYPE.get(self.actor.params);
            let reset = RESET_NO_INPUT.get(p as usize).copied().unwrap_or(0);
            if !(reset == 0 || p == HORSE_PTYPE_4) {
                self.no_input_timer = 0;
                self.no_input_timer_max = 0;
            }
            self.prev_action = self.action;
            self.action = ENHORSE_ACT_FROZEN;
            self.collider_cylinder1.base.oc_flags1 &= !OC1_ON;
            self.collider_cylinder2.base.oc_flags1 &= !OC1_ON;
            self.collider_jnt_sph.base.oc_flags1 &= !OC1_ON;
            self.animation_idx = ENHORSE_ANIM_IDLE;
        }
    }

    /// `EnHorse_Frozen`.
    fn frozen(&mut self, play: &mut PlayState) {
        self.actor.speed_xz = 0.0;
        self.no_input_timer -= 1;
        if self.no_input_timer < 0 {
            self.collider_cylinder1.base.oc_flags1 |= OC1_ON;
            self.collider_cylinder2.base.oc_flags1 |= OC1_ON;
            self.collider_jnt_sph.base.oc_flags1 |= OC1_ON;
            if self.player_controlled {
                self.state_flags &= !ENHORSE_FLAG_7;
                if PTYPE.get(self.actor.params) == HORSE_PTYPE_4 {
                    self.start_mounted_idle_reset_anim(play);
                } else if PTYPE.get(self.actor.params) == HORSE_PTYPE_PLAYER_SPAWNED_RIDING {
                    self.actor.params = HORSE_PTYPE_5;
                    if play.cs_ctx.state != CS_STATE_IDLE {
                        self.start_mounted_idle(play);
                    } else {
                        self.actor.speed_xz = 8.0;
                        self.start_galloping();
                    }
                } else if self.prev_action == 2 {
                    self.start_mounted_idle(play);
                } else {
                    self.start_mounted_idle_reset_anim(play);
                }
                if PTYPE.get(self.actor.params) != HORSE_PTYPE_0 {
                    self.actor.params = HORSE_PTYPE_0;
                }
            } else {
                // (prevAction 5 or 6, or not: the same call.)
                self.change_idle_animation(play, 0, 0.0);
            }
        }
    }

    /// `EnHorse_UpdateSpeed`.
    #[allow(clippy::too_many_arguments)]
    fn update_speed(&mut self, play: &PlayState, brake_decel: f32, brake_angle: f32, min_stick_mag: f32, decel: f32, mut base_speed: f32, turn_speed: i16) {
        if !self.player_can_move(play) {
            if self.actor.speed_xz > 8.0 {
                self.actor.speed_xz -= decel;
            } else if self.actor.speed_xz < 0.0 {
                self.actor.speed_xz = 0.0;
            }
            return;
        }
        base_speed *= self.slope_speed_multiplier();
        let (stick_mag, stick_angle) = stick_direction(self.cur_stick);
        if cos_s(stick_angle) <= brake_angle {
            self.actor.speed_xz -= brake_decel;
            self.actor.speed_xz = self.actor.speed_xz.max(0.0);
            return;
        }
        if stick_mag < min_stick_mag {
            self.state_flags &= !ENHORSE_BOOST;
            self.state_flags &= !ENHORSE_BOOST_DECEL;
            self.actor.speed_xz -= decel;
            if self.actor.speed_xz < 0.0 {
                self.actor.speed_xz = 0.0;
            }
            return;
        }
        if self.state_flags & ENHORSE_BOOST != 0 {
            let mult = self.slope_speed_multiplier();
            if 16 - self.boost_timer > 0 {
                self.actor.speed_xz = (mult * self.boost_speed as f32 - self.actor.speed_xz) / (16 - self.boost_timer) as f32 + self.actor.speed_xz;
            } else {
                self.actor.speed_xz = mult * self.boost_speed as f32;
            }
            if mult * self.boost_speed as f32 <= self.actor.speed_xz {
                self.state_flags &= !ENHORSE_BOOST;
                self.state_flags |= ENHORSE_BOOST_DECEL;
            }
        } else if self.state_flags & ENHORSE_BOOST_DECEL != 0 {
            if base_speed < self.actor.speed_xz {
                self.actor.speed_xz -= 0.06;
            } else if self.actor.speed_xz < base_speed {
                self.actor.speed_xz = base_speed;
                self.state_flags &= !ENHORSE_BOOST_DECEL;
            }
        } else {
            let dir = if self.actor.speed_xz <= base_speed * (1.0 / 54.0) * stick_mag { 1.0 } else { -1.0 };
            self.actor.speed_xz += dir * 50.0 * 0.01;
            if base_speed < self.actor.speed_xz {
                self.actor.speed_xz -= decel;
                if self.actor.speed_xz < base_speed {
                    self.actor.speed_xz = base_speed;
                }
            }
        }
        let temp_f12 = stick_angle as f32 * (1.0 / 32236.0);
        let traction = 2.2 - (self.actor.speed_xz * (1.0 / self.boost_speed as f32));
        let turn = f2s(stick_angle as f32 * temp_f12 * temp_f12 * traction);
        let (lo, hi) = (-(turn_speed as f32) * traction, turn_speed as f32 * traction);
        let turn = if (turn as f32) < lo {
            f2s(lo)
        } else if (turn as f32) > hi {
            f2s(hi)
        } else {
            turn
        };
        self.actor.world_rot.y = self.actor.world_rot.y.wrapping_add(turn);
        self.actor.shape_rot.y = self.actor.world_rot.y;
    }

    /// `EnHorse_StartMountedIdleResetAnim`.
    fn start_mounted_idle_reset_anim(&mut self, play: &mut PlayState) {
        self.skel.cur_frame = 0.0;
        self.start_mounted_idle(play);
        self.state_flags &= !ENHORSE_SANDDUST_SOUND;
    }

    /// `EnHorse_StartMountedIdle`.
    fn start_mounted_idle(&mut self, play: &mut PlayState) {
        self.action = ENHORSE_ACT_MOUNTED_IDLE;
        self.animation_idx = ENHORSE_ANIM_IDLE;
        if ((self.cur_frame > 35.0 && self.ty == HORSE_EPONA) || (self.cur_frame > 28.0 && self.ty == HORSE_HNI)) && self.state_flags & ENHORSE_SANDDUST_SOUND == 0 {
            self.state_flags |= ENHORSE_SANDDUST_SOUND;
            audio_play_actor_sfx2(play, NA_SE_EV_HORSE_SANDDUST);
        }
        let cur = self.skel.cur_frame;
        self.change(1.0, cur, ANIMMODE_ONCE, -3.0);
    }

    /// Where the stick points a mounted horse from idle (`EnHorse_MountedIdle`'s and
    /// `_MountedIdleWhinneying`'s): back reverses, sideways turns, forward walks.
    fn mounted_idle_stick(&mut self, play: &PlayState) {
        let (mag, angle) = stick_direction(self.cur_stick);
        if mag > 10.0 && self.player_can_move(play) {
            if cos_s(angle) <= -0.5 {
                self.start_reversing_interruptable();
            } else if (cos_s(angle) as f64) <= 0.7071 {
                self.start_turning();
            } else {
                self.start_walking_from_idle();
            }
        }
    }

    /// `EnHorse_MountedIdle`.
    fn mounted_idle(&mut self, play: &mut PlayState) {
        self.actor.speed_xz = 0.0;
        self.mounted_idle_stick(play);
        if self.skel.update() {
            // EnHorse_MountedIdleAnim.
            self.skel.cur_frame = 0.0;
            self.mounted_idle_whinney(play);
        }
    }

    /// `EnHorse_MountedIdleWhinney`.
    fn mounted_idle_whinney(&mut self, play: &mut PlayState) {
        self.action = ENHORSE_ACT_MOUNTED_IDLE_WHINNEYING;
        self.animation_idx = ENHORSE_ANIM_WHINNEY;
        let cur = self.skel.cur_frame;
        self.change(1.0, cur, ANIMMODE_ONCE, -3.0);
        self.voice(play, NA_SE_EV_HORSE_GROAN);
    }

    /// `EnHorse_MountedIdleWhinneying`.
    fn mounted_idle_whinneying(&mut self, play: &mut PlayState) {
        self.actor.speed_xz = 0.0;
        self.mounted_idle_stick(play);
        if self.skel.update() {
            self.start_mounted_idle_reset_anim(play);
        }
    }

    /// `EnHorse_StartTurning`.
    fn start_turning(&mut self) {
        self.action = ENHORSE_ACT_MOUNTED_TURN;
        self.sound_timer = 0;
        self.animation_idx = ENHORSE_ANIM_WALK;
        self.change(1.0, 0.0, ANIMMODE_ONCE, -3.0);
    }

    /// `EnHorse_MountedTurn`.
    fn mounted_turn(&mut self, play: &mut PlayState) {
        self.actor.speed_xz = 0.0;
        self.play_walking_sfx(play);
        let (mag, angle) = stick_direction(self.cur_stick);
        if mag > 10.0 {
            if !self.player_can_move(play) {
                self.start_mounted_idle_reset_anim(play);
            } else if cos_s(angle) <= -0.5 {
                self.start_reversing_interruptable();
            } else if (cos_s(angle) as f64) <= 0.7071 {
                let clamped = f2s((angle as f32).clamp(-800.0, 800.0));
                self.actor.world_rot.y = self.actor.world_rot.y.wrapping_add(clamped);
                self.actor.shape_rot.y = self.actor.world_rot.y;
            } else {
                self.start_walking_interruptable();
            }
        }
        if self.skel.update() {
            if (cos_s(angle) as f64) <= 0.7071 {
                self.start_turning();
            } else {
                self.start_mounted_idle_reset_anim(play);
            }
        }
    }

    /// `EnHorse_StartWalkingFromIdle`.
    fn start_walking_from_idle(&mut self) {
        self.start_walking_interruptable();
        if self.state_flags & ENHORSE_FLAG_8 == 0 && self.state_flags & ENHORSE_FLAG_9 == 0 {
            self.state_flags |= ENHORSE_FLAG_9;
            self.wait_timer = 8;
            return;
        }
        self.wait_timer = 0;
    }

    /// `EnHorse_StartWalkingInterruptable`.
    fn start_walking_interruptable(&mut self) {
        self.no_input_timer = 0;
        self.no_input_timer_max = 0;
        self.start_walking();
    }

    /// `EnHorse_StartWalking`.
    fn start_walking(&mut self) {
        self.action = ENHORSE_ACT_MOUNTED_WALK;
        self.sound_timer = 0;
        self.animation_idx = ENHORSE_ANIM_WALK;
        self.wait_timer = 0;
        self.change(1.0, 0.0, ANIMMODE_ONCE, -3.0);
    }

    /// `EnHorse_MountedWalkingReset`.
    fn mounted_walking_reset(&mut self) {
        self.action = ENHORSE_ACT_MOUNTED_WALK;
        self.sound_timer = 0;
        self.animation_idx = ENHORSE_ANIM_WALK;
        self.wait_timer = 0;
        self.play_once();
    }

    /// `EnHorse_MountedWalk`.
    fn mounted_walk(&mut self, play: &mut PlayState) {
        self.play_walking_sfx(play);
        let (mag, angle) = stick_direction(self.cur_stick);
        if self.no_input_timer_max == 0 || (self.no_input_timer > 0 && (self.no_input_timer as f32) < self.no_input_timer_max as f32 - 20.0) {
            self.update_speed(play, 0.3, -0.5, 10.0, 0.06, 3.0, 400);
        } else {
            self.actor.speed_xz = 3.0;
        }
        if self.actor.speed_xz == 0.0 {
            self.state_flags &= !ENHORSE_FLAG_9;
            self.start_mounted_idle_reset_anim(play);
            self.no_input_timer = 0;
            self.no_input_timer_max = 0;
        } else if self.actor.speed_xz > 3.0 {
            self.state_flags &= !ENHORSE_FLAG_9;
            self.start_trotting();
            self.no_input_timer = 0;
            self.no_input_timer_max = 0;
        }
        if self.no_input_timer > 0 {
            self.no_input_timer -= 1;
            if self.no_input_timer <= 0 {
                self.no_input_timer_max = 0;
            }
        }
        if self.wait_timer <= 0 {
            self.state_flags &= !ENHORSE_FLAG_9;
            self.skel.play_speed = self.actor.speed_xz * 0.75;
            if (self.skel.update() || self.actor.speed_xz == 0.0) && self.no_input_timer <= 0 {
                if self.actor.speed_xz > 3.0 {
                    self.start_trotting();
                    self.no_input_timer = 0;
                    self.no_input_timer_max = 0;
                } else if mag < 10.0 || cos_s(angle) <= -0.5 {
                    self.start_mounted_idle_reset_anim(play);
                    self.no_input_timer = 0;
                    self.no_input_timer_max = 0;
                } else {
                    self.mounted_walking_reset();
                }
            }
        } else {
            self.actor.speed_xz = 0.0;
            self.wait_timer -= 1;
        }
    }

    /// `EnHorse_StartTrotting`.
    fn start_trotting(&mut self) {
        self.action = ENHORSE_ACT_MOUNTED_TROT;
        self.animation_idx = ENHORSE_ANIM_TROT;
        self.change(1.0, 0.0, ANIMMODE_ONCE, -3.0);
    }

    /// `EnHorse_MountedTrotReset`.
    fn mounted_trot_reset(&mut self) {
        self.action = ENHORSE_ACT_MOUNTED_TROT;
        self.animation_idx = ENHORSE_ANIM_TROT;
        self.play_once();
    }

    /// `EnHorse_MountedTrot`.
    fn mounted_trot(&mut self, play: &mut PlayState) {
        self.update_speed(play, 0.3, -0.5, 10.0, 0.06, 6.0, 400);
        if self.actor.speed_xz < 3.0 {
            self.start_walking_interruptable();
        }
        self.skel.play_speed = self.actor.speed_xz * 0.375;
        if self.skel.update() {
            Self::play_run_sfx(play);
            // Rumble_Request(0.0f, 60, 8, 255).
            if self.actor.speed_xz >= 6.0 {
                self.start_galloping_interruptable();
            } else if self.actor.speed_xz < 3.0 {
                self.start_walking_interruptable();
            } else {
                self.mounted_trot_reset();
            }
        }
    }

    /// `EnHorse_StartGallopingInterruptable`.
    fn start_galloping_interruptable(&mut self) {
        self.no_input_timer = 0;
        self.no_input_timer_max = 0;
        self.start_galloping();
    }

    /// `EnHorse_StartGalloping`.
    fn start_galloping(&mut self) {
        self.action = ENHORSE_ACT_MOUNTED_GALLOP;
        self.animation_idx = ENHORSE_ANIM_GALLOP;
        self.unk_234 = 0;
        self.change(1.0, 0.0, ANIMMODE_ONCE, -3.0);
    }

    /// `EnHorse_MountedGallopReset`.
    fn mounted_gallop_reset(&mut self) {
        self.no_input_timer = 0;
        self.no_input_timer_max = 0;
        self.action = ENHORSE_ACT_MOUNTED_GALLOP;
        self.animation_idx = ENHORSE_ANIM_GALLOP;
        self.unk_234 = 0;
        self.play_once();
    }

    /// `EnHorse_JumpLanding`.
    fn jump_landing(&mut self) {
        self.action = ENHORSE_ACT_MOUNTED_GALLOP;
        self.animation_idx = ENHORSE_ANIM_GALLOP;
        self.play_once();
        let y = self.root_y();
        self.rider_pos.y += y * 0.01;
        self.post_draw_func = None;
    }

    /// `EnHorse_MountedGallop`.
    fn mounted_gallop(&mut self, play: &mut PlayState) {
        let (mag, angle) = stick_direction(self.cur_stick);
        if self.no_input_timer <= 0 {
            self.update_speed(play, 0.3, -0.5, 10.0, 0.06, 8.0, 0x190);
        } else if self.no_input_timer > 0 {
            self.no_input_timer -= 1;
            self.actor.speed_xz = 8.0;
        }
        if self.actor.speed_xz < 6.0 {
            self.start_trotting();
        }
        self.skel.play_speed = self.actor.speed_xz * 0.3;
        if self.skel.update() {
            Self::play_run_sfx(play);
            // Rumble_Request(0, 120, 8, 255).
            if self.player_can_move(play) {
                if mag >= 10.0 && cos_s(angle) <= -0.5 {
                    self.start_braking(play);
                } else if self.actor.speed_xz < 6.0 {
                    self.start_trotting();
                } else {
                    self.mounted_gallop_reset();
                }
                return;
            }
            self.mounted_gallop_reset();
        }
    }

    /// `EnHorse_StartRearing`.
    fn start_rearing(&mut self, play: &mut PlayState) {
        self.action = ENHORSE_ACT_MOUNTED_REARING;
        self.animation_idx = ENHORSE_ANIM_REARING;
        self.state_flags &= !ENHORSE_LAND2_SOUND;
        self.voice(play, NA_SE_EV_HORSE_NEIGH);
        // Rumble_Request(0.0f, 180, 20, 100).
        self.change(1.0, 0.0, ANIMMODE_ONCE, -3.0);
    }

    /// `EnHorse_MountedRearing`.
    fn mounted_rearing(&mut self, play: &mut PlayState) {
        self.actor.speed_xz = 0.0;
        if self.cur_frame > 25.0 && self.state_flags & ENHORSE_LAND2_SOUND == 0 {
            self.state_flags |= ENHORSE_LAND2_SOUND;
            audio_play_actor_sfx2(play, NA_SE_EV_HORSE_LAND2);
        }
        let (_, angle) = stick_direction(self.cur_stick);
        if self.skel.update() {
            if self.player_can_move(play) {
                if self.state_flags & ENHORSE_FORCE_REVERSING != 0 {
                    self.no_input_timer = 100;
                    self.no_input_timer_max = 100;
                    self.state_flags &= !ENHORSE_FORCE_REVERSING;
                    self.start_reversing();
                } else if self.state_flags & ENHORSE_FORCE_WALKING != 0 {
                    self.no_input_timer = 100;
                    self.no_input_timer_max = 100;
                    self.state_flags &= !ENHORSE_FORCE_WALKING;
                    self.start_walking();
                } else if cos_s(angle) <= -0.5 {
                    self.start_reversing_interruptable();
                } else {
                    self.start_mounted_idle_reset_anim(play);
                }
                return;
            }
            self.start_mounted_idle_reset_anim(play);
        }
    }

    /// `EnHorse_StartBraking`.
    fn start_braking(&mut self, play: &mut PlayState) {
        self.action = ENHORSE_ACT_STOPPING;
        self.animation_idx = ENHORSE_ANIM_STOPPING;
        audio_play_actor_sfx2(play, NA_SE_EV_HORSE_SLIP);
        self.change(1.5, 0.0, ANIMMODE_ONCE, -3.0);
        self.state_flags |= ENHORSE_STOPPING_NEIGH_SOUND;
        self.state_flags &= !ENHORSE_BOOST;
    }

    /// `EnHorse_Stopping`.
    fn stopping(&mut self, play: &mut PlayState) {
        if self.actor.speed_xz > 0.0 {
            self.actor.speed_xz -= 0.6;
            if self.actor.speed_xz < 0.0 {
                self.actor.speed_xz = 0.0;
            }
        }
        if self.state_flags & ENHORSE_STOPPING_NEIGH_SOUND != 0 && self.skel.cur_frame > 29.0 {
            self.actor.speed_xz = 0.0;
            if (play.rand.zero_one() as f64) > 0.5 {
                self.voice(play, NA_SE_EV_HORSE_NEIGH);
                // Rumble_Request(0.0f, 180, 20, 100).
                self.state_flags &= !ENHORSE_STOPPING_NEIGH_SOUND;
            } else {
                self.start_mounted_idle_reset_anim(play);
            }
        }
        if self.skel.cur_frame > 29.0 {
            self.actor.speed_xz = 0.0;
        } else if self.actor.speed_xz > 3.0 && self.state_flags & ENHORSE_FORCE_REVERSING != 0 {
            self.actor.speed_xz = 3.0;
        }
        if self.skel.update() {
            if self.state_flags & ENHORSE_FORCE_REVERSING != 0 {
                self.no_input_timer = 100;
                self.no_input_timer_max = 100;
                self.start_reversing();
                self.state_flags &= !ENHORSE_FORCE_REVERSING;
            } else {
                self.start_mounted_idle_reset_anim(play);
            }
        }
    }

    /// `EnHorse_StartReversingInterruptable`.
    fn start_reversing_interruptable(&mut self) {
        self.no_input_timer = 0;
        self.no_input_timer_max = 0;
        self.start_reversing();
    }

    /// `EnHorse_StartReversing`.
    fn start_reversing(&mut self) {
        self.action = ENHORSE_ACT_REVERSE;
        self.animation_idx = ENHORSE_ANIM_WALK;
        self.sound_timer = 0;
        self.change(1.0, 0.0, ANIMMODE_LOOP, -3.0);
    }

    /// `EnHorse_Reverse`.
    fn reverse(&mut self, play: &mut PlayState) {
        self.play_walking_sfx(play);
        let (mag, mut angle) = stick_direction(self.cur_stick);
        let talking = play.player.and_then(|h| play.actors.actor(h)).is_some_and(|a| a.flags & ACTOR_FLAG_TALK != 0);
        if self.player_can_move(play) {
            if self.no_input_timer_max == 0 || (self.no_input_timer > 0 && (self.no_input_timer as f32) < self.no_input_timer_max as f32 - 20.0) {
                if mag < 10.0 && self.no_input_timer <= 0 {
                    self.start_mounted_idle_reset_anim(play);
                    self.actor.speed_xz = 0.0;
                    return;
                }
                if mag < 10.0 {
                    angle = -0x7FFF;
                } else if cos_s(angle) > -0.5 {
                    self.no_input_timer_max = 0;
                    self.start_mounted_idle_reset_anim(play);
                    self.actor.speed_xz = 0.0;
                    return;
                }
            } else if mag < 10.0 {
                angle = -0x7FFF;
            }
        } else if talking {
            self.start_mounted_idle_reset_anim(play);
            self.actor.speed_xz = 0.0;
            return;
        } else {
            angle = -0x7FFF;
        }
        self.actor.speed_xz = -2.0;
        let turn = (0x7FFF_i32 - angle as i32) as i16;
        let turn = f2s((turn as f32).clamp(-1200.0, 1200.0));
        self.actor.world_rot.y = self.actor.world_rot.y.wrapping_add(turn);
        self.actor.shape_rot.y = self.actor.world_rot.y;
        if self.no_input_timer > 0 {
            self.no_input_timer -= 1;
            if self.no_input_timer <= 0 {
                self.no_input_timer_max = 0;
            }
        }
        self.skel.play_speed = self.actor.speed_xz * 0.5 * 1.5;
        if self.skel.update() && (self.no_input_timer as f32) <= 0.0 && self.player_can_move(play) {
            if mag > 10.0 && cos_s(angle) <= -0.5 {
                self.no_input_timer_max = 0;
                self.start_reversing_interruptable();
            } else if mag < 10.0 {
                self.no_input_timer_max = 0;
                self.start_mounted_idle_reset_anim(play);
            } else {
                self.start_reversing();
            }
        }
    }

    /// `EnHorse_LowJumpInit`.
    fn low_jump_init(&mut self, play: &mut PlayState) {
        self.skel.cur_frame = 0.0;
        self.start_low_jump(play);
    }

    /// `EnHorse_StartLowJump`.
    fn start_low_jump(&mut self, play: &mut PlayState) {
        self.action = ENHORSE_ACT_LOW_JUMP;
        self.animation_idx = ENHORSE_ANIM_LOW_JUMP;
        let cur = self.skel.cur_frame;
        self.change(1.5, cur, ANIMMODE_ONCE, -3.0);
        self.post_draw_func = None;
        self.jump_start_y = self.actor.world_pos.y;
        self.actor.gravity = 0.0;
        self.actor.velocity.y = 0.0;
        let y = self.root_y();
        self.rider_pos.y -= y * 0.01;
        audio_play_actor_sfx2(play, NA_SE_EV_HORSE_JUMP);
        // Rumble_Request(0.0f, 170, 10, 10).
    }

    /// `EnHorse_LowJump`, `EnHorse_HighJump`: the jump's first part follows the root's height
    /// (to frame `split`), then gravity, until the landing.
    fn jump(&mut self, play: &mut PlayState, speed: f32, split: f32, fall: f32) {
        let cur_frame = self.skel.cur_frame;
        self.state_flags |= ENHORSE_JUMPING;
        self.actor.speed_xz = speed;
        if cur_frame > split {
            self.actor.gravity = -3.5;
            if self.actor.velocity.y == 0.0 {
                self.actor.velocity.y = fall;
            }
            if self.actor.world_pos.y < self.actor.floor_height + 90.0 {
                self.skel.play_speed = 1.5;
            } else {
                self.skel.play_speed = 0.0;
            }
        } else {
            let y = self.root_y();
            self.actor.world_pos.y = self.jump_start_y + y * 0.01;
        }
        if self.skel.update() || (cur_frame > split && self.actor.world_pos.y < self.actor.floor_height - self.actor.velocity.y + 80.0) {
            audio_play_actor_sfx2(play, NA_SE_EV_HORSE_LAND);
            // Rumble_Request(0.0f, 255, 10, 80).
            self.state_flags &= !ENHORSE_JUMPING;
            self.actor.gravity = -3.5;
            self.actor.world_pos.y = self.actor.floor_height;
            let p = self.actor.world_pos;
            play.with_ss(|ss| ss.func_80028a54(25.0, p));
            self.jump_landing();
        }
    }

    /// `EnHorse_HighJumpInit`.
    fn high_jump_init(&mut self, play: &mut PlayState) {
        self.skel.cur_frame = 0.0;
        self.start_high_jump(play);
    }

    /// `EnHorse_StartHighJump`.
    fn start_high_jump(&mut self, play: &mut PlayState) {
        self.action = ENHORSE_ACT_HIGH_JUMP;
        self.animation_idx = ENHORSE_ANIM_HIGH_JUMP;
        let cur = self.skel.cur_frame;
        self.change(1.5, cur, ANIMMODE_ONCE, -3.0);
        self.jump_start_y = self.actor.world_pos.y;
        self.post_draw_func = None;
        self.actor.gravity = 0.0;
        self.actor.velocity.y = 0.0;
        let y = self.root_y();
        self.rider_pos.y -= y * 0.01;
        self.state_flags |= ENHORSE_CALC_RIDER_POS;
        audio_play_actor_sfx2(play, NA_SE_EV_HORSE_JUMP);
        // Rumble_Request(0.0f, 170, 10, 10).
    }

    /// `EnHorse_InitInactive`.
    fn init_inactive(&mut self) {
        self.collider_cylinder1.base.oc_flags1 &= !OC1_ON;
        self.collider_cylinder2.base.oc_flags1 &= !OC1_ON;
        self.collider_jnt_sph.base.oc_flags1 &= !OC1_ON;
        self.action = ENHORSE_ACT_INACTIVE;
        self.animation_idx = ENHORSE_ANIM_WALK;
        self.state_flags |= ENHORSE_INACTIVE;
        self.follow_timer = 0;
    }

    /// Epona's Song heard: the camera on her (`Camera_SetViewParam(CAM_VIEW_TARGET)`,
    /// `CAM_SET_TURN_AROUND`, `Camera_SetCameraData(CAM_DATA_SET_2, .., CAM_ITEM_TYPE_81)`).
    fn camera_on_her(play: &mut PlayState) {
        let Some(me) = play.cur_actor else { return };
        let d = play.data.clone();
        play.game_camera.set_target(me);
        play.game_camera.change_setting(&d.camera, CAM_SET_TURN_AROUND);
        // CAM_DATA_SET_2 (4): data2 = CAM_ITEM_TYPE_81.
        play.game_camera.set_camera_data(4, 81, 0);
    }

    /// `EnHorse_Inactive`.
    fn inactive(&mut self, play: &mut PlayState) {
        if play.msg_ctx.r_eponas_song_played && self.ty == HORSE_EPONA {
            play.msg_ctx.r_eponas_song_played = false;
            if self.spawn(play) {
                // (OOT_VERSION >= PAL_1_0.)
                audio_play_actor_sfx2(play, NA_SE_EV_HORSE_NEIGH);
                self.state_flags &= !ENHORSE_INACTIVE;
                play.save.horse_data.scene_id = play.scene_id as i16;
                Self::camera_on_her(play);
            }
        }
        if self.state_flags & ENHORSE_INACTIVE == 0 {
            self.follow_timer = 0;
            self.set_follow_animation(play);
            self.actor.params = HORSE_PTYPE_0;
            self.collider_cylinder1.base.oc_flags1 |= OC1_ON;
            self.collider_cylinder2.base.oc_flags1 |= OC1_ON;
            self.collider_jnt_sph.base.oc_flags1 |= OC1_ON;
        }
    }

    /// `EnHorse_PlayIdleAnimation`.
    fn play_idle_animation(&mut self, play: &mut PlayState, mut anim: i32, morph_frames: f32, start_frame: f32) {
        self.action = ENHORSE_ACT_IDLE;
        self.actor.speed_xz = 0.0;
        if anim != ENHORSE_ANIM_IDLE && anim != ENHORSE_ANIM_WHINNEY && anim != ENHORSE_ANIM_REARING {
            anim = ENHORSE_ANIM_IDLE;
        }
        if anim != self.animation_idx {
            self.animation_idx = anim;
            if self.animation_idx == ENHORSE_ANIM_IDLE {
                self.state_flags &= !ENHORSE_SANDDUST_SOUND;
            } else if self.animation_idx == ENHORSE_ANIM_WHINNEY {
                self.voice(play, NA_SE_EV_HORSE_GROAN);
            } else if self.animation_idx == ENHORSE_ANIM_REARING {
                self.voice(play, NA_SE_EV_HORSE_NEIGH);
                self.state_flags &= !ENHORSE_LAND2_SOUND;
            }
            self.change(1.0, start_frame, ANIMMODE_ONCE, morph_frames);
        }
    }

    /// `EnHorse_ChangeIdleAnimation`.
    fn change_idle_animation(&mut self, play: &mut PlayState, anim: i32, morph_frames: f32) {
        let cur = self.cur_frame;
        self.play_idle_animation(play, anim, morph_frames, cur);
    }

    /// `EnHorse_ResetIdleAnimation`: the walk's index forces the idle to play from its start.
    fn reset_idle_animation(&mut self, play: &mut PlayState) {
        self.animation_idx = ENHORSE_ANIM_WALK;
        self.play_idle_animation(play, self.animation_idx, 0.0, 0.0);
    }

    /// `EnHorse_StartIdleRidable`.
    fn start_idle_ridable(&mut self, play: &mut PlayState) {
        self.reset_idle_animation(play);
        self.state_flags &= !ENHORSE_UNRIDEABLE;
    }

    /// `EnHorse_Idle`.
    fn idle(&mut self, play: &mut PlayState) {
        self.actor.speed_xz = 0.0;
        self.idle_anim_sounds(play);
        if play.msg_ctx.r_eponas_song_played && self.ty == HORSE_EPONA {
            play.msg_ctx.r_eponas_song_played = false;
            if !self.func_80a5bbbc(play, self.actor.world_pos) {
                if self.spawn(play) {
                    // (OOT_VERSION >= PAL_1_0.)
                    audio_play_actor_sfx2(play, NA_SE_EV_HORSE_NEIGH);
                    self.follow_timer = 0;
                    self.set_follow_animation(play);
                    Self::camera_on_her(play);
                }
            } else {
                audio_play_actor_sfx2(play, NA_SE_EV_HORSE_NEIGH);
                self.follow_timer = 0;
                self.start_moving_animation(6, -3.0, 0.0);
            }
        }
        if self.skel.update() {
            let idle_anim_idx = if self.animation_idx == ENHORSE_ANIM_WHINNEY {
                1
            } else if self.animation_idx == ENHORSE_ANIM_REARING {
                2
            } else {
                0
            };
            // Play one of the two other idle animations.
            let r = if play.rand.zero_one() > 0.5 { 0 } else { 1 };
            self.play_idle_animation(play, IDLE_ANIM_IDS[(r + idle_anim_idx * 2) as usize], 0.0, 0.0);
        }
    }

    /// `EnHorse_StartMovingAnimation`.
    fn start_moving_animation(&mut self, mut anim_id: i32, morph_frames: f32, start_frame: f32) {
        self.action = ENHORSE_ACT_FOLLOW_PLAYER;
        self.state_flags &= !ENHORSE_TURNING_TO_PLAYER;
        if anim_id != ENHORSE_ANIM_TROT && anim_id != ENHORSE_ANIM_GALLOP && anim_id != ENHORSE_ANIM_WALK {
            anim_id = ENHORSE_ANIM_WALK;
        }
        if self.animation_idx != anim_id {
            self.animation_idx = anim_id;
            self.change(1.0, start_frame, ANIMMODE_ONCE, morph_frames);
        } else {
            self.change(1.0, start_frame, ANIMMODE_ONCE, 0.0);
        }
    }

    /// `EnHorse_SetFollowAnimation`.
    fn set_follow_animation(&mut self, play: &PlayState) {
        let pp = Self::player_pos(play);
        let dist = ((pp.x - self.actor.world_pos.x).powi(2) + (pp.z - self.actor.world_pos.z).powi(2)).sqrt();
        let mut anim_id = ENHORSE_ANIM_WALK;
        if dist > 400.0 {
            anim_id = ENHORSE_ANIM_GALLOP;
        } else if !(dist <= 300.0) && dist <= 400.0 {
            anim_id = ENHORSE_ANIM_TROT;
        }
        if self.animation_idx == ENHORSE_ANIM_GALLOP {
            anim_id = if dist > 400.0 { ENHORSE_ANIM_GALLOP } else { ENHORSE_ANIM_TROT };
        } else if self.animation_idx == ENHORSE_ANIM_TROT {
            anim_id = if dist > 400.0 {
                ENHORSE_ANIM_GALLOP
            } else if dist < 300.0 {
                ENHORSE_ANIM_WALK
            } else {
                ENHORSE_ANIM_TROT
            };
        } else if self.animation_idx == ENHORSE_ANIM_WALK {
            anim_id = if dist > 300.0 { ENHORSE_ANIM_TROT } else { ENHORSE_ANIM_WALK };
        }
        self.start_moving_animation(anim_id, -3.0, 0.0);
    }

    /// `EnHorse_FollowPlayer`.
    fn follow_player(&mut self, play: &mut PlayState) {
        play.msg_ctx.r_eponas_song_played = false;
        let pp = Self::player_pos(play);
        let dist = ((pp.x - self.actor.world_pos.x).powi(2) + (pp.z - self.actor.world_pos.z).powi(2)).sqrt();
        // First rotate if the player is behind.
        if (self.player_dir == PLAYER_DIR_BACK_R || self.player_dir == PLAYER_DIR_BACK_L) && dist > 300.0 && self.state_flags & ENHORSE_TURNING_TO_PLAYER == 0 {
            self.animation_idx = ENHORSE_ANIM_REARING;
            self.state_flags |= ENHORSE_TURNING_TO_PLAYER;
            self.angle_to_player = yaw_to(self.actor.world_pos, pp);
            let mut angle_diff = self.angle_to_player as f32 - self.actor.world_rot.y as f32;
            if angle_diff > 32767.0 {
                angle_diff -= 32767.0;
            } else if angle_diff < -32767.0 {
                angle_diff += 32767.0;
            }
            self.follow_player_turn_speed = f2s(angle_diff / self.anim(self.animation_idx).last_frame());
            self.play_once();
            self.skel.play_speed = 1.0;
            self.state_flags &= !ENHORSE_LAND2_SOUND;
            self.unk_21c = self.unk_228;
            // (OOT_VERSION < PAL_1_0: a neigh here.)
        } else if self.state_flags & ENHORSE_TURNING_TO_PLAYER != 0 {
            self.actor.world_rot.y = self.actor.world_rot.y.wrapping_add(self.follow_player_turn_speed);
            self.actor.shape_rot.y = self.actor.world_rot.y;
            if self.cur_frame > 25.0 && self.state_flags & ENHORSE_LAND2_SOUND == 0 {
                self.state_flags |= ENHORSE_LAND2_SOUND;
                audio_play_actor_sfx2(play, NA_SE_EV_HORSE_LAND2);
            }
        } else {
            self.rotate_to_player(play);
        }

        if self.animation_idx == ENHORSE_ANIM_GALLOP {
            self.actor.speed_xz = 8.0;
            self.skel.play_speed = self.actor.speed_xz * 0.3;
        } else if self.animation_idx == ENHORSE_ANIM_TROT {
            self.actor.speed_xz = 6.0;
            self.skel.play_speed = self.actor.speed_xz * 0.375;
        } else if self.animation_idx == ENHORSE_ANIM_WALK {
            self.actor.speed_xz = 3.0;
            self.play_walking_sfx(play);
            self.skel.play_speed = self.actor.speed_xz * 0.75;
        } else {
            self.actor.speed_xz = 0.0;
            self.skel.play_speed = 1.0;
        }

        if self.state_flags & ENHORSE_TURNING_TO_PLAYER == 0 {
            self.follow_timer += 1;
            if self.follow_timer > 300 {
                self.start_idle_ridable(play);
                self.voice(play, NA_SE_EV_HORSE_NEIGH);
            }
        }

        if self.skel.update() {
            if self.animation_idx == ENHORSE_ANIM_GALLOP || self.animation_idx == ENHORSE_ANIM_TROT {
                Self::play_run_sfx(play);
            }
            self.state_flags &= !ENHORSE_TURNING_TO_PLAYER;
            if dist < 100.0 {
                self.start_idle_ridable(play);
            } else {
                self.set_follow_animation(play);
            }
        }
    }

    /// `EnHorse_InitIngoHorse`.
    fn init_ingo_horse(&mut self) {
        self.cur_race_waypoint = 0;
        self.sound_timer = 0;
        self.actor.speed_xz = 0.0;
        self.update_ingo_horse_anim(None);
        self.unk_21c = self.unk_228;
        // (Its init: not drawn yet, so no NA_SE_IT_INGO_HORSE_NEIGH.)
    }

    /// `EnHorse_SetIngoAnimation`: Ingo's animation for the horse's, and its frame.
    fn set_ingo_animation(index: i32, cur_frame: f32, is_lashing: bool) -> (i16, f32) {
        let mut anim = INGO_ANIMATIONS[index as usize];
        let mut frame = cur_frame;
        if index == ENHORSE_ANIM_REARING || index == ENHORSE_ANIM_LOW_JUMP || index == ENHORSE_ANIM_HIGH_JUMP || index == ENHORSE_ANIM_WALK {
            frame = 0.0;
        }
        if is_lashing {
            if index == ENHORSE_ANIM_TROT {
                anim = ENIN_HORSE_ANIM_LASHING_SLOW;
                frame = cur_frame;
            } else if index == ENHORSE_ANIM_GALLOP {
                anim = ENIN_HORSE_ANIM_LASHING_FAST;
                frame = cur_frame;
            }
        }
        (anim, frame)
    }

    /// `EnHorse_UpdateIngoHorseAnim`.
    fn update_ingo_horse_anim(&mut self, play: Option<&mut PlayState>) {
        let mut anim_changed = false;
        self.action = ENHORSE_ACT_INGO_RACE;
        self.state_flags &= !ENHORSE_SANDDUST_SOUND;
        let new = if self.actor.speed_xz == 0.0 {
            ENHORSE_ANIM_IDLE
        } else if self.actor.speed_xz <= 3.0 {
            ENHORSE_ANIM_WALK
        } else if self.actor.speed_xz <= 6.0 {
            ENHORSE_ANIM_TROT
        } else {
            ENHORSE_ANIM_GALLOP
        };
        if self.animation_idx != new {
            anim_changed = true;
        }
        self.animation_idx = new;
        let anim_speed = match self.animation_idx {
            ENHORSE_ANIM_WALK => self.actor.speed_xz * 0.5,
            ENHORSE_ANIM_TROT => self.actor.speed_xz * 0.25,
            ENHORSE_ANIM_GALLOP => self.actor.speed_xz * 0.2,
            _ => 1.0,
        };
        if (self.animation_idx == ENHORSE_ANIM_TROT || self.animation_idx == ENHORSE_ANIM_GALLOP)
            && let Some(play) = play
        {
            audio_play_actor_sfx2(play, NA_SE_EV_HORSE_RUN);
        }
        let speed = PLAYBACK_SPEEDS[self.animation_idx as usize] * anim_speed * 1.5;
        self.change(speed, 0.0, ANIMMODE_ONCE, if anim_changed { -3.0 } else { 0.0 });
    }

    /// `EnHorse_UpdateIngoRace`.
    fn update_ingo_race(&mut self, play: &mut PlayState) {
        if self.animation_idx == ENHORSE_ANIM_IDLE || self.animation_idx == ENHORSE_ANIM_WHINNEY {
            self.idle_anim_sounds(play);
        } else if self.animation_idx == ENHORSE_ANIM_WALK {
            self.play_walking_sfx(play);
        }
        self.update_ingo_race_info(play);
        if !self.in_race {
            self.actor.speed_xz = 0.0;
            if let Some(r) = self.rider.and_then(|h| play.actors.actor_mut(h)) {
                r.speed_xz = 0.0;
            }
            if self.animation_idx != ENHORSE_ANIM_IDLE {
                self.update_ingo_horse_anim(Some(play));
            }
        }
        self.skel.play_speed = match self.animation_idx {
            ENHORSE_ANIM_WALK => self.actor.speed_xz * 0.5,
            ENHORSE_ANIM_TROT => self.actor.speed_xz * 0.25,
            ENHORSE_ANIM_GALLOP => self.actor.speed_xz * 0.2,
            _ => 1.0,
        };
        if self.skel.update() || (self.animation_idx == ENHORSE_ANIM_IDLE && self.actor.speed_xz != 0.0) {
            self.update_ingo_horse_anim(Some(play));
        }
        let (anim, frame) = if self.state_flags & ENHORSE_INGO_WON != 0 {
            (ENIN_HORSE_ANIM_WHOOPING, 0.0)
        } else {
            Self::set_ingo_animation(self.animation_idx, self.skel.cur_frame, self.ingo_race_flags & INGO_FLAG_LASHING != 0)
        };
        if let Some(h) = self.rider {
            crate::en_in::set_horseback_animation(play, h, anim, frame);
        }
    }

    /// `EnHorse_CsMoveInit`.
    fn cs_move_init(&mut self) {
        self.animation_idx = ENHORSE_ANIM_GALLOP;
        self.cutscene_action = 1;
        let s = self.actor.speed_xz * 0.3;
        self.play_once_set_speed(s);
    }

    /// `EnHorse_CsMoveToPoint`, `EnHorse_CsWarpMoveToPoint`.
    fn cs_move_to_point(&mut self, play: &mut PlayState, cue: &CsCmdActorCue) {
        let end = Vec3::new(cue.end_pos.x as f32, cue.end_pos.y as f32, cue.end_pos.z as f32);
        let speed_xz = 8.0;
        if end.distance(self.actor.world_pos) > speed_xz {
            self.rotate_to_point(end, 400);
            self.actor.speed_xz = speed_xz;
            self.skel.play_speed = speed_xz * 0.3;
        } else {
            self.actor.world_pos = end;
            self.actor.speed_xz = 0.0;
        }
        if self.skel.update() {
            Self::play_run_sfx(play);
            // Rumble_Request(0.0f, 120, 8, 255).
            let s = self.actor.speed_xz * 0.3;
            self.play_once_set_speed(s);
        }
    }

    /// `EnHorse_CsPlayHighJumpAnim`.
    fn cs_play_high_jump_anim(&mut self, play: &mut PlayState) {
        self.animation_idx = ENHORSE_ANIM_HIGH_JUMP;
        let cur = self.skel.cur_frame;
        self.change(1.5, cur, ANIMMODE_ONCE, -3.0);
        self.post_draw_func = None;
        self.jump_start_y = self.actor.world_pos.y;
        self.actor.gravity = 0.0;
        self.actor.velocity.y = 0.0;
        let y = self.root_y();
        self.rider_pos.y -= y * 0.01;
        self.state_flags |= ENHORSE_CALC_RIDER_POS;
        audio_play_actor_sfx2(play, NA_SE_EV_HORSE_JUMP);
        // Rumble_Request(0.0f, 170, 10, 10).
    }

    /// `EnHorse_CsJumpInit` (`EnHorse_CsSetAnimHighJump`).
    fn cs_jump_init(&mut self, play: &mut PlayState) {
        self.skel.cur_frame = 0.0;
        self.cs_play_high_jump_anim(play);
        self.cutscene_action = 2;
        self.cutscene_flags &= !1;
    }

    /// `EnHorse_CsJump`.
    fn cs_jump(&mut self, play: &mut PlayState, cue: &CsCmdActorCue) {
        if self.cutscene_flags & 1 != 0 {
            self.cs_move_to_point(play, cue);
            return;
        }
        let temp_f2 = self.skel.cur_frame;
        self.state_flags |= ENHORSE_JUMPING;
        self.actor.speed_xz = 13.0;
        if temp_f2 > 19.0 {
            self.actor.gravity = -3.5;
            if self.actor.velocity.y == 0.0 {
                self.actor.velocity.y = -10.5;
            }
            self.skel.play_speed = if self.actor.world_pos.y < self.actor.floor_height + 90.0 { 1.5 } else { 0.0 };
        } else {
            let y = self.root_y();
            self.actor.world_pos.y = self.jump_start_y + y * 0.01;
        }
        if self.skel.update() || (temp_f2 > 19.0 && self.actor.world_pos.y < (self.actor.floor_height - self.actor.velocity.y) + 80.0) {
            self.cutscene_flags |= 1;
            audio_play_actor_sfx2(play, NA_SE_EV_HORSE_LAND);
            // Rumble_Request(0.0f, 255, 10, 80).
            self.state_flags &= !ENHORSE_JUMPING;
            self.actor.gravity = -3.5;
            self.actor.velocity.y = 0.0;
            self.actor.world_pos.y = self.actor.floor_height;
            let p = self.actor.world_pos;
            play.with_ss(|ss| ss.func_80028a54(25.0, p));
            self.animation_idx = ENHORSE_ANIM_GALLOP;
            self.play_once_set_speed(PLAYBACK_SPEEDS[6]);
            let y = self.root_y();
            self.rider_pos.y += y * 0.01;
            self.post_draw_func = None;
        }
    }

    /// `EnHorse_CsRearingInit`, `EnHorse_CsWarpRearingInit`'s shared part.
    fn cs_rearing_start(&mut self, play: &mut PlayState, action: i32) {
        self.animation_idx = ENHORSE_ANIM_REARING;
        self.cutscene_action = action;
        self.cutscene_flags &= !4;
        self.state_flags &= !ENHORSE_LAND2_SOUND;
        self.voice(play, NA_SE_EV_HORSE_NEIGH);
        self.change(1.0, 0.0, ANIMMODE_ONCE, -3.0);
    }

    /// `EnHorse_CsRearing`, `EnHorse_CsWarpRearing`.
    fn cs_rearing(&mut self, play: &mut PlayState) {
        self.actor.speed_xz = 0.0;
        if self.cur_frame > 25.0 && self.state_flags & ENHORSE_LAND2_SOUND == 0 {
            self.state_flags |= ENHORSE_LAND2_SOUND;
            audio_play_actor_sfx2(play, NA_SE_EV_HORSE_LAND2);
        }
        if self.skel.update() {
            self.animation_idx = ENHORSE_ANIM_IDLE;
            if self.cutscene_flags & 4 == 0 {
                self.cutscene_flags |= 4;
                self.change(1.0, 0.0, ANIMMODE_ONCE, -3.0);
            } else {
                // (Mode 0: ANIMMODE_LOOP.)
                self.change(1.0, 0.0, ANIMMODE_LOOP, 0.0);
            }
        }
    }

    /// `EnHorse_WarpMoveInit`, `EnHorse_CsWarpRearingInit`'s start: at the cue's start.
    fn cs_warp_to_start(&mut self, cue: &CsCmdActorCue) {
        self.actor.world_pos = Vec3::new(cue.start_pos.x as f32, cue.start_pos.y as f32, cue.start_pos.z as f32);
        self.actor.prev_pos = self.actor.world_pos;
        self.actor.world_rot.y = cue.rot[1];
        self.actor.shape_rot = self.actor.world_rot;
    }

    /// `EnHorse_InitCutscene`.
    fn init_cutscene(&mut self) {
        self.player_controlled = false;
        self.action = ENHORSE_ACT_CS_UPDATE;
        self.cutscene_action = 0;
        self.actor.speed_xz = 0.0;
    }

    /// `EnHorse_GetCutsceneFunctionIndex`.
    fn get_cutscene_function_index(cue_id: u16) -> i32 {
        for &(id, idx) in CS_ACTION_TABLE.iter() {
            if cue_id == id {
                return idx;
            }
            if cue_id < id {
                return 0;
            }
        }
        0
    }

    /// `EnHorse_CutsceneUpdate`.
    fn cutscene_update(&mut self, play: &mut PlayState) {
        if play.cs_ctx.state == 3 {
            self.player_controlled = true;
            self.actor.params = HORSE_PTYPE_10;
            self.action = ENHORSE_ACT_IDLE;
            self.freeze();
            return;
        }
        let Some(cue) = play.cs_ctx.link_action else { return };
        let idx = Self::get_cutscene_function_index(cue.action);
        if idx == 0 {
            return;
        }
        if self.cutscene_action != idx {
            if self.cutscene_action == 0 {
                self.actor.world_pos = Vec3::new(cue.start_pos.x as f32, cue.start_pos.y as f32, cue.start_pos.z as f32);
                self.actor.world_rot.y = cue.rot[1];
                self.actor.shape_rot = self.actor.world_rot;
                self.actor.prev_pos = self.actor.world_pos;
            }
            self.cutscene_action = idx;
            match idx {
                1 => self.cs_move_init(),
                2 => self.cs_jump_init(play),
                3 => self.cs_rearing_start(play, 3),
                4 => {
                    // EnHorse_WarpMoveInit.
                    self.cs_warp_to_start(&cue);
                    self.animation_idx = ENHORSE_ANIM_GALLOP;
                    self.cutscene_action = 4;
                    let s = self.actor.speed_xz * 0.3;
                    self.play_once_set_speed(s);
                }
                _ => {
                    // EnHorse_CsWarpRearingInit.
                    self.cs_warp_to_start(&cue);
                    self.cs_rearing_start(play, 5);
                }
            }
        }
        match self.cutscene_action {
            1 | 4 => self.cs_move_to_point(play, &cue),
            2 => self.cs_jump(play, &cue),
            _ => self.cs_rearing(play),
        }
    }

    /// `EnHorse_UpdateHorsebackArchery` (Phase 18): logged once; the horse stands.
    fn update_horseback_archery(&mut self) {
        if !self.logged_hba {
            self.logged_hba = true;
            log::info!("En_Horse: EnHorse_UpdateHorsebackArchery isn't ported (Phase 18)");
        }
        self.actor.speed_xz = 0.0;
        self.skel.update();
    }

    /// `EnHorse_InitFleePlayer`.
    fn init_flee_player(&mut self) {
        self.action = ENHORSE_ACT_FLEE_PLAYER;
        self.state_flags |= ENHORSE_UNRIDEABLE;
        self.actor.speed_xz = 0.0;
    }

    /// `EnHorse_FleePlayer`.
    fn flee_player(&mut self, play: &mut PlayState) {
        let next_anim;
        if play.msg_ctx.r_eponas_song_played || self.ty == HORSE_HNI {
            self.start_idle_ridable(play);
            audio_play_actor_sfx2(play, NA_SE_EV_HORSE_NEIGH);
        }
        let pp = Self::player_pos(play);
        let player_shape_yaw = play.player.and_then(|h| play.actors.actor(h)).map(|a| a.shape_rot.y).unwrap_or(0);
        let dist_to_home = self.actor.home_pos.distance(self.actor.world_pos);
        let player_dist_to_home = pp.distance(self.actor.home_pos);
        let dist_to_player = pp.distance(self.actor.world_pos);

        if player_dist_to_home > 300.0 {
            // Run home.
            if dist_to_home > 150.0 {
                self.actor.speed_xz += 0.4;
                if self.actor.speed_xz > 8.0 {
                    self.actor.speed_xz = 8.0;
                }
            } else {
                self.actor.speed_xz -= 0.47;
                if self.actor.speed_xz < 0.0 {
                    self.actor.speed_xz = 0.0;
                }
            }
        } else if dist_to_player < 300.0 {
            // Run away from Link.
            self.actor.speed_xz += 0.4;
            if self.actor.speed_xz > 8.0 {
                self.actor.speed_xz = 8.0;
            }
        } else {
            self.actor.speed_xz -= 0.47;
            if self.actor.speed_xz < 0.0 {
                self.actor.speed_xz = 0.0;
            }
        }

        if self.actor.speed_xz >= 6.0 {
            self.skel.play_speed = self.actor.speed_xz * 0.3;
            next_anim = ENHORSE_ANIM_GALLOP;
        } else if self.actor.speed_xz >= 3.0 {
            self.skel.play_speed = self.actor.speed_xz * 0.375;
            next_anim = ENHORSE_ANIM_TROT;
        } else if self.actor.speed_xz > 0.1 {
            self.skel.play_speed = self.actor.speed_xz * 0.75;
            next_anim = ENHORSE_ANIM_WALK;
            self.play_walking_sfx(play);
        } else {
            next_anim = if play.rand.zero_one() > 0.5 { 1 } else { 0 };
            self.idle_anim_sounds(play);
            self.skel.play_speed = 1.0;
        }

        // Turn away from Link, or towards home.
        if next_anim == ENHORSE_ANIM_GALLOP || next_anim == ENHORSE_ANIM_TROT || next_anim == ENHORSE_ANIM_WALK {
            let yaw: i16 = if player_dist_to_home < 300.0 {
                let toward = yaw_to(self.actor.world_pos, pp);
                // (yaw += (s16)(sign * 0x3FFF): s16 arithmetic.)
                player_shape_yaw.wrapping_add(if toward > 0 { 0x3FFF } else { -0x3FFF })
            } else {
                yaw_to(self.actor.world_pos, self.actor.home_pos).wrapping_sub(self.actor.world_rot.y)
            };
            if yaw > 400 {
                self.actor.world_rot.y = self.actor.world_rot.y.wrapping_add(400);
            } else if yaw < -400 {
                self.actor.world_rot.y = self.actor.world_rot.y.wrapping_sub(400);
            } else {
                self.actor.world_rot.y = self.actor.world_rot.y.wrapping_add(yaw);
            }
            self.actor.shape_rot.y = self.actor.world_rot.y;
        }

        let anim_finished = self.skel.update();
        let moving = |a: i32| a == ENHORSE_ANIM_GALLOP || a == ENHORSE_ANIM_TROT || a == ENHORSE_ANIM_WALK;
        if (self.animation_idx == ENHORSE_ANIM_IDLE || self.animation_idx == ENHORSE_ANIM_WHINNEY) && moving(next_anim) {
            self.animation_idx = next_anim;
            self.change(1.0, 0.0, ANIMMODE_ONCE, -3.0);
            if self.animation_idx == ENHORSE_ANIM_GALLOP || self.animation_idx == ENHORSE_ANIM_TROT {
                Self::play_run_sfx(play);
            }
            return;
        }
        if anim_finished {
            if next_anim == ENHORSE_ANIM_GALLOP || next_anim == ENHORSE_ANIM_TROT {
                Self::play_run_sfx(play);
            }
            if self.animation_idx == ENHORSE_ANIM_IDLE || self.animation_idx == ENHORSE_ANIM_WHINNEY {
                if next_anim != self.animation_idx {
                    self.animation_idx = next_anim;
                    self.change(1.0, 0.0, ANIMMODE_ONCE, -3.0);
                    return;
                }
                if play.rand.zero_one() > 0.5 {
                    self.animation_idx = ENHORSE_ANIM_IDLE;
                    self.state_flags &= !ENHORSE_SANDDUST_SOUND;
                } else {
                    self.animation_idx = ENHORSE_ANIM_WHINNEY;
                    self.voice(play, NA_SE_EV_HORSE_GROAN);
                }
                self.change(1.0, 0.0, ANIMMODE_ONCE, -3.0);
                return;
            }
            if next_anim != self.animation_idx {
                self.animation_idx = next_anim;
                self.change(1.0, 0.0, ANIMMODE_ONCE, -3.0);
            } else {
                self.change(1.0, 0.0, ANIMMODE_ONCE, 0.0);
            }
            return;
        }
        if self.animation_idx == ENHORSE_ANIM_WALK && (next_anim == ENHORSE_ANIM_IDLE || next_anim == ENHORSE_ANIM_WHINNEY) {
            self.animation_idx = next_anim;
            self.change(1.0, 0.0, ANIMMODE_ONCE, -3.0);
        }
    }

    /// `EnHorse_BridgeJumpInit` (Phase 18's jump; reached only from its script's start).
    fn bridge_jump_init(&mut self, play: &mut PlayState) {
        let p = self.actor.world_pos;
        play.with_ss(|ss| ss.func_80028a54(25.0, p));
        self.action = ENHORSE_ACT_BRIDGE_JUMP;
        self.state_flags |= ENHORSE_JUMPING;
        self.animation_idx = ENHORSE_ANIM_HIGH_JUMP;
        let y = self.root_y() * 0.01;
        self.bridge_jump_start = self.actor.world_pos;
        self.bridge_jump_start.y += y;
        let b = &BRIDGE_JUMPS[self.bridge_jump_idx as usize];
        self.bridge_jump_y_vel = (((b.pos[1] as f32 + 48.7) - self.bridge_jump_start.y) - -360.0) / 30.0;
        self.rider_pos.y -= y;
        self.state_flags |= ENHORSE_CALC_RIDER_POS;
        self.bridge_jump_rel_angle = self.actor.world_rot.y.wrapping_sub(b.angle);
        self.bridge_jump_timer = 0;
        self.actor.gravity = 0.0;
        self.actor.speed_xz = 0.0;
        self.change(1.5, 0.0, ANIMMODE_ONCE, -3.0);
        self.voice(play, NA_SE_EV_HORSE_NEIGH);
        audio_play_actor_sfx2(play, NA_SE_EV_HORSE_JUMP);
        // Rumble_Request(0.0f, 170, 10, 10).
        self.post_draw_func = None;
    }

    /// `EnHorse_BridgeJump` (`EnHorse_BridgeJumpMove`, `EnHorse_CheckBridgeJumpLanding`).
    fn bridge_jump(&mut self, play: &mut PlayState) {
        self.bridge_jump_timer += 1;
        let b = &BRIDGE_JUMPS[self.bridge_jump_idx as usize];
        if self.bridge_jump_timer < 30 {
            let interp = self.bridge_jump_timer as f32 / 30.0;
            let time_sq = (self.bridge_jump_timer * self.bridge_jump_timer) as f32;
            let s = self.bridge_jump_start;
            self.actor.world_pos.x = ((b.pos[0] as f32 - s.x) * interp) + s.x;
            self.actor.world_pos.z = ((b.pos[2] as f32 - s.z) * interp) + s.z;
            self.actor.world_pos.y = s.y + (self.bridge_jump_y_vel * self.bridge_jump_timer as f32) + (-0.4 * time_sq);
            let yaw = f2s(b.angle as f32 + ((1.0 - interp) * self.bridge_jump_rel_angle as f32));
            self.actor.world_rot.y = yaw;
            self.actor.shape_rot.y = yaw;
            self.skel.cur_frame = 23.0 * interp;
            self.skel.update();
            if self.bridge_jump_timer < 30 {
                self.state_flags |= ENHORSE_FLAG_24;
            }
            return;
        }
        self.actor.speed_xz = 8.0;
        self.skel.play_speed = 1.5;
        if self.skel.update() {
            self.state_flags &= !ENHORSE_JUMPING;
            self.actor.gravity = -3.5;
            self.actor.world_pos.y = b.pos[1] as f32;
            let p = self.actor.world_pos;
            play.with_ss(|ss| ss.func_80028a54(25.0, p));
            self.jump_landing();
            audio_play_actor_sfx2(play, NA_SE_EV_HORSE_LAND);
            // Rumble_Request(0.0f, 255, 10, 80).
        }
    }

    /// `EnHorse_CalcFloorHeight`: 0 a floor, 1 none, 2 water above it, 3 a blocked floor (steeper
    /// than 35 degrees, horse-blocked or `FLOOR_TYPE_7`).
    fn calc_floor_height(play: &PlayState, pos: Vec3) -> (i32, Option<PolyId>, f32) {
        let (y, poly) = play.col.entity_raycast_down(pos);
        if y == BGCHECK_Y_MIN {
            return (1, None, y);
        }
        if let Some(w) = play.col.water_surface(pos.x, pos.z, play.room_ctx.cur.num as u8 as u32)
            && y < w
        {
            return (2, poly, y);
        }
        let Some(p) = poly else { return (1, None, y) };
        if play.col.poly_normal(p).y < 0.819_152_06 || play.col.is_horse_blocked(p) || play.col.floor_type(p) == FLOOR_TYPE_7 {
            return (3, poly, y);
        }
        (0, poly, y)
    }

    /// `EnHorse_ObstructMovement` (obstacle 1 water ahead, 2 water behind, 4 in front, 5 behind).
    fn obstruct_movement(&mut self, play: &mut PlayState, obstacle_type: i32, galloping: bool) {
        if self.action == ENHORSE_ACT_CS_UPDATE || self.bg_check_bridge_jump_point(play) {
            return;
        }
        self.actor.world_pos = self.last_pos;
        self.actor.shape_rot.y = self.last_yaw;
        self.actor.world_rot.y = self.last_yaw;
        self.state_flags |= ENHORSE_OBSTACLE;
        if !self.player_controlled {
            if self.animation_idx != ENHORSE_ANIM_REARING {
                return;
            }
        } else if self.action != ENHORSE_ACT_MOUNTED_REARING {
            if self.state_flags & ENHORSE_JUMPING != 0 {
                self.state_flags &= !ENHORSE_JUMPING;
                self.actor.gravity = -3.5;
                self.actor.world_pos.y = self.actor.floor_height;
            }
            if obstacle_type == 1 || obstacle_type == 4 {
                self.state_flags |= ENHORSE_FORCE_REVERSING;
            } else if obstacle_type == 2 || obstacle_type == 5 {
                self.state_flags |= ENHORSE_FORCE_WALKING;
            }
            if galloping {
                self.start_rearing(play);
            }
        }
    }

    /// `EnHorse_CheckFloors`.
    fn check_floors(&mut self, play: &mut PlayState) {
        let galloping = self.actor.speed_xz > 8.0;
        if let Some(w) = play.col.water_surface(self.actor.world_pos.x, self.actor.world_pos.z, play.room_ctx.cur.num as u8 as u32)
            && self.actor.floor_height < w
        {
            self.obstruct_movement(play, 1, galloping);
            return;
        }
        let front_pos = vec3f_offset(self.actor.world_pos, self.actor.shape_rot.y, 30.0, 60.0);
        let (status, front_floor, yf) = Self::calc_floor_height(play, front_pos);
        self.y_front = yf;
        if status != 0 {
            if status == 1 {
                self.actor.shape_rot.x = 0;
            }
            self.obstruct_movement(play, 4, galloping);
            return;
        }
        let back_pos = vec3f_offset(self.actor.world_pos, self.actor.shape_rot.y, -30.0, 60.0);
        let (status, back_floor, yb) = Self::calc_floor_height(play, back_pos);
        self.y_back = yb;
        if status != 0 {
            if status == 1 {
                self.actor.shape_rot.x = 0;
            }
            self.obstruct_movement(play, 5, galloping);
            return;
        }
        let floor_slope = atan2_binang(self.y_back - self.y_front, 60.0);
        let Some(fp) = self.actor.floor_poly else { return };
        let n = play.col.poly_normal(fp);
        let dist = play.col.poly(fp).dist as f32;
        let mut pos = front_pos;
        pos.y = self.y_front;
        let d = dist_plane_to_pos(n, dist, pos);
        if front_floor != Some(fp) && self.actor.speed_xz >= 0.0 && ((self.state_flags & ENHORSE_JUMPING == 0 && d < -40.0) || (self.state_flags & ENHORSE_JUMPING != 0 && d < -200.0)) {
            self.obstruct_movement(play, 4, galloping);
            return;
        }
        let mut pos = back_pos;
        pos.y = self.y_back;
        let d = dist_plane_to_pos(n, dist, pos);
        if (back_floor != Some(fp) && self.actor.speed_xz <= 0.0 && self.state_flags & ENHORSE_JUMPING == 0 && d < -40.0) || (self.state_flags & ENHORSE_JUMPING != 0 && d < -200.0) {
            self.obstruct_movement(play, 5, galloping);
            return;
        }
        if n.y < 0.819_152_06 || play.col.is_horse_blocked(fp) || play.col.floor_type(fp) == FLOOR_TYPE_7 {
            if self.actor.speed_xz >= 0.0 {
                self.obstruct_movement(play, 4, galloping);
            } else {
                self.obstruct_movement(play, 5, galloping);
            }
            return;
        }
        if self.state_flags & ENHORSE_JUMPING != 0 {
            self.actor.shape_rot.x = 0;
            return;
        }
        if self.actor.floor_height + 4.0 < self.actor.world_pos.y {
            self.actor.shape_rot.x = 0;
            return;
        }
        if (floor_slope as f32).abs() > 8191.0 {
            return;
        }
        self.actor.shape_rot.x = floor_slope;
        self.actor.shape_y_offset = (self.y_front + (((self.y_back - self.y_front) * 20.0) / 45.0)) - self.actor.floor_height;
    }

    /// `EnHorse_PlayerDirToMountSide`, `EnHorse_MountSideCheck`, `EnHorse_GetMountSide`: -1 Link at
    /// her left, 1 her right, 0 no mounting.
    fn get_mount_side(&self, play: &PlayState) -> i32 {
        if self.action != ENHORSE_ACT_IDLE {
            return 0;
        }
        if self.animation_idx != ENHORSE_ANIM_IDLE && self.animation_idx != ENHORSE_ANIM_WHINNEY {
            return 0;
        }
        let Some(p) = play.player.and_then(|h| play.actors.actor(h)) else { return 0 };
        let dx = p.world_pos.x - self.actor.world_pos.x;
        let dz = p.world_pos.z - self.actor.world_pos.z;
        if (dx * dx + dz * dz).sqrt() > 75.0 {
            return 0;
        }
        if (self.actor.world_pos.y - p.world_pos.y).abs() > 30.0 {
            return 0;
        }
        if cos_s(yaw_to(p.world_pos, self.actor.world_pos).wrapping_sub(p.world_rot.y)) < 0.173_648_18 {
            return 0;
        }
        match self.player_dir {
            PLAYER_DIR_SIDE_L => -1,
            PLAYER_DIR_SIDE_R => 1,
            _ => 0,
        }
    }

    /// `EnHorse_MountDismount`.
    fn mount_dismount(&mut self, play: &mut PlayState) {
        let side = self.get_mount_side(play);
        let me = play.cur_actor;
        if side != 0
            && self.state_flags & ENHORSE_UNRIDEABLE == 0
            && let Some(me) = me
            && let Some(p) = play.player.and_then(|h| play.actors.get_mut(h)).and_then(|p| p.as_player_mut())
            && p.ride_actor().is_none()
        {
            // Actor_SetRideActor.
            p.set_ride_actor(me, side);
        }
        if !self.player_controlled && actor_is_mounted(&self.actor) {
            self.no_input_timer = 55;
            self.no_input_timer_max = 55;
            self.player_controlled = true;
            self.freeze();
        } else if self.player_controlled && actor_not_mounted(&self.actor) {
            self.no_input_timer = 35;
            self.no_input_timer_max = 35;
            self.state_flags &= !ENHORSE_UNRIDEABLE;
            self.player_controlled = false;
            self.freeze();
        }
    }

    /// `EnHorse_UpdateStick`: the pad's stick (`rel`).
    fn update_stick(&mut self, play: &PlayState) {
        self.last_stick = self.cur_stick;
        self.cur_stick = Vec2::new(play.input.rel.stick_x as f32, play.input.rel.stick_y as f32);
    }

    /// `EnHorse_ResolveCollision`: pushed off a wall faced within 45 degrees, to 30 from it.
    fn resolve_collision(&mut self, play: &PlayState, wall: PolyId) {
        let raw = play.col.poly(wall).normal;
        let n = play.col.poly_normal(wall);
        let face = atan2_binang(raw[0] as f32, raw[2] as f32);
        if !(cos_s(self.actor.world_rot.y.wrapping_sub(face).wrapping_sub(0x7FFF)) < 0.7071) {
            let dist = dist_plane_to_pos(n, play.col.poly(wall).dist as f32, self.actor.world_pos);
            let mut offset = 1.0 / (n.x * n.x + n.z * n.z).sqrt();
            offset *= 30.0 - dist;
            self.actor.world_pos.x += offset * n.x;
            self.actor.world_pos.z += offset * n.z;
        }
    }

    /// `EnHorse_BgCheckSlowMoving`.
    fn bg_check_slow_moving(&mut self, play: &PlayState) {
        let y_offset = if play.scene_id == SCENE_LON_LON_RANCH { 19.0 } else { 40.0 };
        let mut start = self.actor.world_pos;
        start.y += y_offset;
        let mut end = start;
        end.x += 30.0 * sin_s(self.actor.world_rot.y);
        end.y += 30.0 * sin_s(self.actor.shape_rot.x.wrapping_neg());
        end.z += 30.0 * cos_s(self.actor.world_rot.y);
        if let Some((_, poly)) = play.col.entity_line_test(start, end, true, false, false, true) {
            self.resolve_collision(play, poly);
        }
    }

    /// `EnHorse_UpdateBgCheckInfo`: the walls, then what's ahead at a gallop (jumps).
    fn update_bg_check_info(&mut self, play: &mut PlayState) {
        let h = if play.scene_id == SCENE_LON_LON_RANCH { 19.0 } else { 40.0 };
        self.actor.update_bg_check_info(&play.col, h, 35.0, 100.0, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2 | UPDBGCHECKINFO_FLAG_3 | UPDBGCHECKINFO_FLAG_4);
        if self.bg_check_bridge_jump_point(play) {
            return;
        }
        if self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 && cos_s(self.actor.wall_yaw.wrapping_sub(self.actor.world_rot.y)) < -0.3 && self.actor.speed_xz > 4.0 {
            self.actor.speed_xz -= 1.0;
            audio_play_actor_sfx2(play, NA_SE_EV_HORSE_SANDDUST);
        }
        if self.state_flags & ENHORSE_JUMPING != 0 || !self.player_controlled {
            return;
        }
        if self.actor.speed_xz < 0.0 {
            return;
        }
        // Braking or rearing from an obstacle.
        if self.action == ENHORSE_ACT_STOPPING || self.action == ENHORSE_ACT_MOUNTED_REARING {
            return;
        }
        let (mut intersect_dist, moving_fast) = if self.actor.speed_xz > 8.0 {
            if self.actor.speed_xz < 12.8 { (160.0_f32, false) } else { (230.0, true) }
        } else {
            self.bg_check_slow_moving(play);
            return;
        };
        let mut start_pos = self.actor.world_pos;
        start_pos.y += 19.0;
        let mut end_pos = start_pos;
        end_pos.x += intersect_dist * sin_s(self.actor.world_rot.y);
        end_pos.y += intersect_dist * sin_s(self.actor.shape_rot.x.wrapping_neg());
        end_pos.z += intersect_dist * cos_s(self.actor.world_rot.y);
        let mut intersect = end_pos;
        let mut wall = None;
        if let Some((hit, poly)) = play.col.entity_line_test(start_pos, end_pos, true, false, false, true) {
            intersect = hit;
            wall = Some(poly);
            // (f32)sqrt(Math3D_Vec3fDistSq): a double square root.
            intersect_dist = (start_pos.distance_squared(intersect) as f64).sqrt() as f32;
            self.state_flags |= ENHORSE_OBSTACLE;
        }
        if let Some(w) = wall {
            if intersect_dist < 30.0 {
                self.resolve_collision(play, w);
            }
            let raw = play.col.poly(w).normal;
            let face = atan2_binang(raw[0] as f32, raw[2] as f32);
            if cos_s(self.actor.world_rot.y.wrapping_sub(face).wrapping_sub(0x7FFF)) < 0.5 || play.col.is_horse_blocked(w) {
                return;
            }
            // Too close to jump.
            if (!moving_fast && intersect_dist < 80.0) || (moving_fast && intersect_dist < 150.0) {
                self.state_flags |= ENHORSE_FORCE_REVERSING;
                if moving_fast {
                    self.start_braking(play);
                }
                return;
            }
            let is_umajump = !w.is_scene() && dyna_actor_id(play, w.bg) == Some(crate::bg_umajump::ACTOR_BG_UMAJUMP);
            if self.state_flags & ENHORSE_FLAG_26 != 0 && !is_umajump {
                self.state_flags |= ENHORSE_FORCE_REVERSING;
                if moving_fast {
                    self.start_braking(play);
                }
                return;
            }
        }
        // The obstacle's height.
        intersect_dist += 5.0;
        let mut obstacle_pos = start_pos;
        obstacle_pos.x += intersect_dist * sin_s(self.actor.world_rot.y);
        obstacle_pos.y = self.actor.world_pos.y + 120.0;
        obstacle_pos.z += intersect_dist * cos_s(self.actor.world_rot.y);
        let (top_y, obstacle_floor) = play.col.entity_raycast_down(obstacle_pos);
        if top_y == BGCHECK_Y_MIN {
            return;
        }
        let mut obstacle_top = obstacle_pos;
        obstacle_top.y = top_y;
        let obstacle_height = obstacle_top.y - self.actor.world_pos.y;
        let (Some(fp), Some(of)) = (self.actor.floor_poly, obstacle_floor) else { return };
        let fn_ = play.col.poly_normal(fp);
        let on = play.col.poly_normal(of);
        if dist_plane_to_pos(fn_, play.col.poly(fp).dist as f32, obstacle_top) < -40.0 && dist_plane_to_pos(on, play.col.poly(of).dist as f32, self.actor.world_pos) > 40.0 {
            if moving_fast && self.action != ENHORSE_ACT_STOPPING {
                self.state_flags |= ENHORSE_FORCE_REVERSING;
                self.start_braking(play);
            }
            self.state_flags |= ENHORSE_OBSTACLE;
            return;
        }
        if on.y < 0.819_152_06 || play.col.is_horse_blocked(of) || play.col.floor_type(of) == FLOOR_TYPE_7 {
            if moving_fast && self.action != ENHORSE_ACT_STOPPING {
                self.state_flags |= ENHORSE_FORCE_REVERSING;
                self.start_braking(play);
            }
            return;
        }
        if wall.is_none() || obstacle_top.y < intersect.y || self.state_flags & ENHORSE_CANT_JUMP != 0 {
            return;
        }
        let mut obstacle_pos = start_pos;
        obstacle_pos.y = self.actor.world_pos.y + 120.0;
        let ahead = if !moving_fast { 276.0 } else { 390.0 };
        obstacle_pos.x += ahead * sin_s(self.actor.world_rot.y);
        obstacle_pos.z += ahead * cos_s(self.actor.world_rot.y);
        let (beyond_y, beyond_floor) = play.col.entity_raycast_down(obstacle_pos);
        if beyond_y == BGCHECK_Y_MIN {
            return;
        }
        let behind_obstacle_height = beyond_y - self.actor.world_pos.y;
        let Some(bf) = beyond_floor else { return };
        let bn = play.col.poly_normal(bf);
        if bn.y < 0.819_152_06 || play.col.is_horse_blocked(bf) || play.col.floor_type(bf) == FLOOR_TYPE_7 {
            if moving_fast && self.action != ENHORSE_ACT_STOPPING {
                self.state_flags |= ENHORSE_FORCE_REVERSING;
                self.start_braking(play);
            }
        } else if behind_obstacle_height < -(DREG_4 as f32) {
            // -70.
            if moving_fast && self.action != ENHORSE_ACT_STOPPING {
                self.state_flags |= ENHORSE_FORCE_REVERSING;
                self.start_braking(play);
            }
        } else if !moving_fast && obstacle_height > 19.0 && obstacle_height <= 40.0 {
            // EnHorse_Stub1.
            self.post_draw_func = Some(PostDraw::LowJumpInit);
        } else if (moving_fast && self.actor.speed_xz < 13.8 && obstacle_height > 19.0 && obstacle_height <= 72.0) || (self.actor.speed_xz > 13.8 && obstacle_height <= 112.0) {
            // EnHorse_Stub2.
            self.post_draw_func = Some(PostDraw::HighJumpInit);
        }
    }

    /// `EnHorse_CheckBoost`: A ("Faster") spurs a walking, trotting or galloping horse.
    fn check_boost(&mut self, play: &mut PlayState) {
        if (self.action == ENHORSE_ACT_MOUNTED_WALK || self.action == ENHORSE_ACT_MOUNTED_TROT || self.action == ENHORSE_ACT_MOUNTED_GALLOP)
            && play.input.press.held(eng_input::pad::BTN_A)
            && play.interface_ctx.unk_1ee == oot_game::interface::DO_ACTION_FASTER
            && self.state_flags & ENHORSE_BOOST == 0
            && self.state_flags & ENHORSE_FLAG_8 == 0
            && self.state_flags & ENHORSE_FLAG_9 == 0
        {
            if self.num_boosts > 0 {
                // Rumble_Request(0.0f, 180, 20, 100).
                self.state_flags |= ENHORSE_BOOST;
                self.state_flags |= ENHORSE_FIRST_BOOST_REGEN;
                self.state_flags |= ENHORSE_FLAG_8;
                self.num_boosts -= 1;
                self.boost_timer = 0;
                if self.num_boosts == 0 {
                    self.boost_regen_time = 140;
                    return;
                }
                if self.ty == HORSE_EPONA {
                    if self.state_flags & ENHORSE_FIRST_BOOST_REGEN != 0 {
                        self.boost_regen_time = 60;
                        self.state_flags &= !ENHORSE_FIRST_BOOST_REGEN;
                    } else {
                        // Never happens.
                        self.boost_regen_time = 8;
                    }
                } else {
                    self.boost_regen_time = 70;
                }
                return;
            }
            self.unk_21c = self.unk_228;
            if self.state_flags & ENHORSE_DRAW != 0 && play.rand.zero_one() < 0.1 {
                audio_play_actor_sfx2(play, NA_SE_EV_HORSE_NEIGH);
            }
        }
    }

    /// `EnHorse_RegenBoost`: the carrots come back one by one (or all after the last), and the
    /// HUD's count.
    fn regen_boost(&mut self, play: &mut PlayState) {
        if self.num_boosts < 6 && self.num_boosts > 0 {
            self.boost_regen_time -= 1;
            self.boost_timer += 1;
            if self.boost_regen_time <= 0 {
                self.num_boosts += 1;
                if !self.check_4() {
                    play.audio.play_sfx_centered(NA_SE_SY_CARROT_RECOVER);
                }
                if self.num_boosts < 6 {
                    self.boost_regen_time = 11;
                }
            }
        } else if self.num_boosts == 0 {
            self.boost_regen_time -= 1;
            self.boost_timer += 1;
            if self.boost_regen_time <= 0 {
                self.boost_regen_time = 0;
                self.num_boosts = 6;
                if !self.check_4() {
                    play.audio.play_sfx_centered(NA_SE_SY_CARROT_RECOVER);
                }
            }
        }
        if self.boost_timer == 8 && play.rand.zero_one() < 0.25 {
            self.voice(play, NA_SE_EV_HORSE_NEIGH);
        }
        play.interface_ctx.num_horse_boosts = self.num_boosts;
    }

    /// `EN_HORSE_CHECK_4`: standing (mounted idle, frozen or whinnying), not in the race and not
    /// shut in.
    pub fn check_4(&self) -> bool {
        (self.action == ENHORSE_ACT_MOUNTED_IDLE || self.action == ENHORSE_ACT_FROZEN || self.action == ENHORSE_ACT_MOUNTED_IDLE_WHINNEYING)
            && self.state_flags & ENHORSE_FLAG_19 == 0
            && self.state_flags & ENHORSE_FLAG_25 == 0
    }

    /// `EnHorse_UpdatePlayerDir`.
    fn update_player_dir(&mut self, play: &PlayState) {
        let pp = Self::player_pos(play);
        let angle = yaw_to(self.actor.world_pos, pp).wrapping_sub(self.actor.world_rot.y);
        let s = sin_s(angle);
        let c = cos_s(angle);
        self.player_dir = if s > 0.866_025_4 {
            PLAYER_DIR_SIDE_L
        } else if s < -0.866_025_4 {
            PLAYER_DIR_SIDE_R
        } else if c > 0.0 {
            if s > 0.0 { PLAYER_DIR_FRONT_L } else { PLAYER_DIR_FRONT_R }
        } else if s > 0.0 {
            PLAYER_DIR_BACK_L
        } else {
            PLAYER_DIR_BACK_R
        };
    }

    /// `EnHorse_TiltBody`: the roll into a turn, by speed.
    fn tilt_body(&mut self) {
        let speed = self.actor.speed_xz / self.boost_speed as f32;
        let turn_vel = self.actor.shape_rot.y.wrapping_sub(self.last_yaw);
        let target_roll = -(f2s((1820.0 * speed) * (turn_vel as f32 / 480.000_03)) as i32);
        let roll_diff = target_roll as f32 - self.actor.world_rot.z as f32;
        if (target_roll as f32).abs() < 100.0 {
            self.actor.world_rot.z = 0;
        } else if roll_diff.abs() < 100.0 {
            self.actor.world_rot.z = target_roll as i16;
        } else if roll_diff > 0.0 {
            self.actor.world_rot.z = self.actor.world_rot.z.wrapping_add(100);
        } else {
            self.actor.world_rot.z = self.actor.world_rot.z.wrapping_sub(100);
        }
        self.actor.shape_rot.z = self.actor.world_rot.z;
    }

    /// `EnHorse_UpdateConveyors`.
    fn update_conveyors(&mut self, play: &PlayState) -> bool {
        let rides = play.player.and_then(|h| play.actors.get(h)).and_then(|p| p.as_player()).and_then(|p| p.ride_actor()) == play.cur_actor;
        let Some(fp) = self.actor.floor_poly else {
            if !rides {
                return false;
            }
            // (SurfaceType_GetConveyorDirection of a NULL poly: the C reads the scene's first
            // poly's; a ridden horse always has a floor here.)
            return true;
        };
        let dir = play.col.conveyor_direction(fp);
        // CONVEYOR_DIRECTION_TO_BINANG: (dir << 10).
        let conveyor_dir = ((dir << 10) as i16).wrapping_sub(self.actor.world_rot.y);
        if conveyor_dir as f32 > 800.0 {
            self.actor.world_rot.y = f2s(self.actor.world_rot.y as f32 + 800.0);
        } else if (conveyor_dir as f32) < -800.0 {
            self.actor.world_rot.y = f2s(self.actor.world_rot.y as f32 - 800.0);
        } else {
            self.actor.world_rot.y = self.actor.world_rot.y.wrapping_add(conveyor_dir);
        }
        self.actor.shape_rot.y = self.actor.world_rot.y;
        true
    }

    /// The post-draw's matrices: `Skin_ApplyAnimTransformations` as `EnHorse_Draw` applies it.
    fn place(&self) -> SkinPlace {
        SkinPlace { scale: self.actor.scale, rot: [self.actor.shape_rot.x, self.actor.shape_rot.y, self.actor.shape_rot.z], pos: self.actor.world_pos, y_offset: self.actor.shape_y_offset }
    }

    /// `EnHorse_RandomOffset`.
    fn random_offset(play: &mut PlayState, src: Vec3, dist: f32) -> Vec3 {
        let x = (play.rand.zero_one() * (dist * 2.0) + src.x) - dist;
        let y = (play.rand.zero_one() * (dist * 2.0) + src.y) - dist;
        let z = (play.rand.zero_one() * (dist * 2.0) + src.z) - dist;
        Vec3::new(x, y, z)
    }

    /// `EnHorse_PostDraw`.
    fn post_draw(&mut self, play: &mut PlayState) {
        let set_translation = self.state_flags & ENHORSE_JUMPING == 0;
        let (mats, skin_mtx) = apply_anim_transformations(&self.skeleton, &self.skel.joint_table, &self.place(), set_translation);
        let limb_pos = |limb: usize, off: Vec3| get_limb_pos(&skin_mtx, &mats, limb, off);
        let hoof_offset = Vec3::new(5.0, -4.0, 5.0);
        let frame = self.skel.cur_frame;
        if self.state_flags & ENHORSE_CALC_RIDER_POS == 0 {
            let p = limb_pos(30, Vec3::new(600.0, -1670.0, 0.0));
            self.rider_pos = p - self.actor.world_pos;
        } else {
            self.state_flags &= !ENHORSE_CALC_RIDER_POS;
        }
        let head = limb_pos(13, Vec3::ZERO);
        self.unk_228 = (play.view_proj * head.extend(1.0)).truncate();
        if (self.animation_idx == ENHORSE_ANIM_IDLE && self.action != ENHORSE_ACT_FROZEN)
            && ((frame > 40.0 && frame < 45.0 && self.ty == HORSE_EPONA) || (frame > 28.0 && frame < 33.0 && self.ty == HORSE_HNI))
        {
            if play.rand.zero_one() < 0.6 {
                self.dust_flags |= 1;
                self.front_right_hoof = limb_pos(28, hoof_offset);
                self.front_right_hoof.y -= 5.0;
            }
        } else if self.action == ENHORSE_ACT_STOPPING {
            if (frame > 10.0 && frame < 13.0) || (frame > 25.0 && frame < 33.0) {
                if play.rand.zero_one() < 0.7 {
                    self.dust_flags |= 2;
                    let p = limb_pos(20, hoof_offset);
                    self.front_left_hoof = Self::random_offset(play, p, 10.0);
                }
                if play.rand.zero_one() < 0.7 {
                    self.dust_flags |= 1;
                    let p = limb_pos(28, hoof_offset);
                    self.front_right_hoof = Self::random_offset(play, p, 10.0);
                }
            }
            if ((frame > 6.0 && frame < 10.0) || (frame > 23.0 && frame < 29.0)) && play.rand.zero_one() < 0.7 {
                self.dust_flags |= 8;
                let p = limb_pos(37, hoof_offset);
                self.back_left_hoof = Self::random_offset(play, p, 10.0);
            }
            if ((frame > 7.0 && frame < 14.0) || (frame > 26.0 && frame < 30.0)) && play.rand.zero_one() < 0.7 {
                self.dust_flags |= 4;
                let p = limb_pos(45, hoof_offset);
                self.back_right_hoof = Self::random_offset(play, p, 10.0);
            }
        } else if self.animation_idx == ENHORSE_ANIM_GALLOP {
            if frame > 14.0 && frame < 16.0 {
                self.dust_flags |= 1;
                let p = limb_pos(28, hoof_offset);
                self.front_right_hoof = Self::random_offset(play, p, 5.0);
            } else if frame > 8.0 && frame < 10.0 {
                self.dust_flags |= 2;
                let p = limb_pos(20, hoof_offset);
                self.front_left_hoof = Self::random_offset(play, p, 10.0);
            } else if frame > 1.0 && frame < 3.0 {
                self.dust_flags |= 4;
                let p = limb_pos(45, hoof_offset);
                self.back_right_hoof = Self::random_offset(play, p, 10.0);
            } else if frame > 26.0 && frame < 28.0 {
                self.dust_flags |= 8;
                let p = limb_pos(37, hoof_offset);
                self.back_left_hoof = Self::random_offset(play, p, 10.0);
            }
        } else if self.action == ENHORSE_ACT_LOW_JUMP && frame > 6.0 && play.rand.zero_one() < 1.0 - (frame - 6.0) * (1.0 / 17.0) {
            self.back_hooves_dust(play, &limb_pos, hoof_offset);
        } else if self.action == ENHORSE_ACT_HIGH_JUMP && frame > 5.0 && play.rand.zero_one() < 1.0 - (frame - 5.0) * (1.0 / 25.0) {
            self.back_hooves_dust(play, &limb_pos, hoof_offset);
        } else if self.action == ENHORSE_ACT_BRIDGE_JUMP && play.rand.zero_one() < 0.5 {
            if play.rand.zero_one() < 0.5 {
                self.dust_flags |= 8;
                let p = limb_pos(37, hoof_offset);
                self.back_left_hoof = Self::random_offset(play, p, 10.0);
            } else {
                self.dust_flags |= 4;
                let p = limb_pos(45, hoof_offset);
                self.back_right_hoof = Self::random_offset(play, p, 10.0);
            }
        }
        for e in &mut self.collider_jnt_sph.elements {
            let c = e.dim.model_sphere.center;
            let center = Vec3::new(c[0] as f32, c[1] as f32, c[2] as f32);
            let new_center = get_limb_pos(&skin_mtx, &mats, e.dim.limb as usize, center);
            e.dim.world_sphere.center = [new_center.x as i16, new_center.y as i16, new_center.z as i16];
            e.dim.world_sphere.radius = (e.dim.model_sphere.radius as f32 * e.dim.scale) as i16;
        }
        // @bug (game): set from the draw, which can add the entries twice (a pause).
        play.collision_check_set_oc(&self.actor, 2, &mut self.collider_jnt_sph);
        play.collision_check_set_ac(&self.actor, 2, &mut self.collider_jnt_sph);
    }

    /// The low and high jumps' back hooves' dust (each at a 1 in 2 chance).
    fn back_hooves_dust(&mut self, play: &mut PlayState, limb_pos: &dyn Fn(usize, Vec3) -> Vec3, hoof_offset: Vec3) {
        if play.rand.zero_one() < 0.5 {
            self.dust_flags |= 8;
            let p = limb_pos(37, hoof_offset);
            self.back_left_hoof = Self::random_offset(play, p, 10.0);
        }
        if play.rand.zero_one() < 0.5 {
            self.dust_flags |= 4;
            let p = limb_pos(45, hoof_offset);
            self.back_right_hoof = Self::random_offset(play, p, 10.0);
        }
    }

    /// `EnHorse_RandInt`.
    fn rand_int(play: &mut PlayState, range: f32) -> i32 {
        (play.rand.zero_one() * range) as i32
    }
}

/// `DREG(4)`: 70, set by the init when zero.
const DREG_4: i16 = 70;
/// `FLOOR_TYPE_7` (`bgcheck.h`).
const FLOOR_TYPE_7: u32 = 7;

/// `EnHorse_StickDirection`: the stick's magnitude (to 60) and angle.
pub fn stick_direction(cur_stick: Vec2) -> (f32, i16) {
    let (x, y) = (cur_stick.x, cur_stick.y);
    let dist = (x * x + y * y).sqrt();
    let mag = if dist > 60.0 { 60.0 } else { dist };
    (mag, atan2_binang(-x, y))
}

/// `GET_EVENTCHKINF_CARPENTERS_ALL_RESCUED()` (`save.h`): `EVENTCHKINF_CARPENTERS_FREE` 0x90 to
/// 0x93 all set.
fn carpenters_all_rescued(play: &PlayState) -> bool {
    // schema: EVENTCHKINF_CARPENTERS_ALL_RESCUED (0x90 to 0x93), read whole.
    EVENTCHKINF_CARPENTERS_ALL_RESCUED_FLAGS.iter().all(|&f| play.save.get_event_chk_inf(f))
}

impl ActorImpl for EnHorse {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnHorse_Update`.
    fn update(&mut self, play: &mut PlayState) {
        self.last_yaw = self.actor.shape_rot.y;
        self.update_stick(play);
        self.update_player_dir(play);
        if self.state_flags & ENHORSE_INACTIVE == 0 {
            self.mount_dismount(play);
        }
        if self.state_flags & ENHORSE_FLAG_19 != 0 {
            if self.state_flags & ENHORSE_FLAG_20 != 0 && self.in_race {
                self.state_flags &= !ENHORSE_FLAG_20;
                self.start_rearing(play);
            } else if self.state_flags & ENHORSE_FLAG_20 == 0 && self.state_flags & ENHORSE_FLAG_21 != 0 && self.action != ENHORSE_ACT_MOUNTED_REARING && self.in_race {
                self.state_flags &= !ENHORSE_FLAG_21;
                self.start_rearing(play);
            }
        }
        match self.action {
            ENHORSE_ACT_FROZEN => self.frozen(play),
            ENHORSE_ACT_INACTIVE => self.inactive(play),
            ENHORSE_ACT_IDLE => self.idle(play),
            ENHORSE_ACT_FOLLOW_PLAYER => self.follow_player(play),
            ENHORSE_ACT_INGO_RACE => self.update_ingo_race(play),
            ENHORSE_ACT_MOUNTED_IDLE => self.mounted_idle(play),
            ENHORSE_ACT_MOUNTED_IDLE_WHINNEYING => self.mounted_idle_whinneying(play),
            ENHORSE_ACT_MOUNTED_TURN => self.mounted_turn(play),
            ENHORSE_ACT_MOUNTED_WALK => self.mounted_walk(play),
            ENHORSE_ACT_MOUNTED_TROT => self.mounted_trot(play),
            ENHORSE_ACT_MOUNTED_GALLOP => self.mounted_gallop(play),
            ENHORSE_ACT_MOUNTED_REARING => self.mounted_rearing(play),
            ENHORSE_ACT_STOPPING => self.stopping(play),
            ENHORSE_ACT_REVERSE => self.reverse(play),
            ENHORSE_ACT_LOW_JUMP => self.jump(play, 12.0, 17.0, -6.0),
            ENHORSE_ACT_HIGH_JUMP => self.jump(play, 13.0, 23.0, -10.5),
            ENHORSE_ACT_BRIDGE_JUMP => self.bridge_jump(play),
            ENHORSE_ACT_CS_UPDATE => self.cutscene_update(play),
            ENHORSE_ACT_HBA => self.update_horseback_archery(),
            _ => self.flee_player(play),
        }
        self.state_flags &= !ENHORSE_OBSTACLE;
        self.cur_frame = self.skel.cur_frame;
        self.last_pos = self.actor.world_pos;
        if self.state_flags & ENHORSE_INACTIVE != 0 {
            return;
        }
        if self.action == ENHORSE_ACT_MOUNTED_GALLOP || self.action == ENHORSE_ACT_MOUNTED_TROT || self.action == ENHORSE_ACT_MOUNTED_WALK {
            self.check_boost(play);
        }
        if self.player_controlled {
            self.regen_boost(play);
        }
        // Actor_MoveXZGravity.
        self.actor.move_forward();
        if self.action == ENHORSE_ACT_INGO_RACE
            && let Some(r) = self.rider.and_then(|h| play.actors.actor_mut(h))
        {
            r.world_pos.x = self.actor.world_pos.x;
            r.world_pos.y = self.actor.world_pos.y + 10.0;
            r.world_pos.z = self.actor.world_pos.z;
            r.shape_rot.x = self.actor.shape_rot.x;
            r.shape_rot.y = self.actor.shape_rot.y;
        }
        if self.collider_jnt_sph.elements[0].info.oc_elem_flags & OCELEM_HIT != 0 && self.actor.speed_xz > 6.0 {
            self.actor.speed_xz -= 1.0;
        }
        if self.collider_jnt_sph.base.ac_flags & AC_HIT != 0 {
            self.voice(play, NA_SE_EV_HORSE_NEIGH);
        }
        if self.action != ENHORSE_ACT_INGO_RACE {
            self.tilt_body();
        }
        self.collider_cylinder1.update(&self.actor);
        self.collider_cylinder2.update(&self.actor);
        // (Required to match: the s16 sums.)
        let yaw = self.actor.shape_rot.y;
        self.collider_cylinder1.dim.pos[0] = self.collider_cylinder1.dim.pos[0].wrapping_add(f2s(sin_s(yaw) * 11.0));
        self.collider_cylinder1.dim.pos[2] = self.collider_cylinder1.dim.pos[2].wrapping_add(f2s(cos_s(yaw) * 11.0));
        self.collider_cylinder2.dim.pos[0] = self.collider_cylinder2.dim.pos[0].wrapping_add(f2s(sin_s(yaw) * -18.0));
        self.collider_cylinder2.dim.pos[2] = self.collider_cylinder2.dim.pos[2].wrapping_add(f2s(cos_s(yaw) * -18.0));
        play.collision_check_set_at(&self.actor, 0, &mut self.collider_cylinder1);
        play.collision_check_set_oc(&self.actor, 0, &mut self.collider_cylinder1);
        play.collision_check_set_oc(&self.actor, 1, &mut self.collider_cylinder2);
        let (p_state1, p_rides) = play
            .player
            .and_then(|h| play.actors.get(h))
            .and_then(|p| p.as_player())
            .map(|p| (p.state_flags1(), p.ride_actor().is_some()))
            .unwrap_or((0, false));
        if p_state1 & PLAYER_STATE1_0 != 0 && p_rides && (play.scene_id != SCENE_LON_LON_RANCH || self.actor.world_pos.z < -2400.0) {
            self.update_conveyors(play);
        }
        if self.state_flags & ENHORSE_FLAG_24 == 0 {
            self.update_bg_check_info(play);
            self.check_floors(play);
            if self.actor.world_pos.y < self.y_front && self.actor.world_pos.y < self.y_back {
                self.actor.world_pos.y = if self.y_back < self.y_front { self.y_back } else { self.y_front };
            }
        } else {
            self.state_flags &= !ENHORSE_FLAG_24;
        }
        if play.scene_id == SCENE_GERUDO_VALLEY && !carpenters_all_rescued(play) {
            self.check_bridge_jumps(play);
        }
        self.actor.focus_pos = self.actor.world_pos + Vec3::Y * 70.0;
        if play.rand.zero_one() < 0.025 && self.blink_timer == 0 {
            self.blink_timer += 1;
        } else if self.blink_timer > 0 {
            self.blink_timer += 1;
            if self.blink_timer >= 4 {
                self.blink_timer = 0;
            }
        }
        self.actor.col_chk_info.mass = if self.actor.speed_xz == 0.0 && self.state_flags & ENHORSE_FLAG_19 == 0 { MASS_IMMOVABLE } else { MASS_HEAVY };
        if self.actor.speed_xz >= 5.0 {
            self.collider_cylinder1.base.at_flags |= AT_ON;
        } else {
            self.collider_cylinder1.base.at_flags &= !AT_ON;
        }
        if play.save.entrance_index != ENTR_LON_LON_RANCH_0 || play.save.scene_layer != 9 {
            let dust = |play: &mut PlayState, pos: Vec3| {
                let scale = Self::rand_int(play, 100.0) + 200;
                let step = Self::rand_int(play, 10.0) + 30;
                let life = Self::rand_int(play, 20.0) + 30;
                play.with_ss(|ss| ss.func_800287ac(pos, Vec3::new(0.0, 1.0, 0.0), Vec3::ZERO, scale as i16, step as i16, life as i16));
            };
            if self.dust_flags & 1 != 0 {
                self.dust_flags &= !1;
                dust(play, self.front_right_hoof);
            } else if self.dust_flags & 2 != 0 {
                self.dust_flags &= !2;
                dust(play, self.front_left_hoof);
            } else if self.dust_flags & 4 != 0 {
                self.dust_flags &= !4;
                dust(play, self.back_right_hoof);
            } else if self.dust_flags & 8 != 0 {
                self.dust_flags &= !8;
                dust(play, self.back_left_hoof);
            }
        }
        self.state_flags &= !ENHORSE_DRAW;
    }

    /// `EnHorse_Draw`'s state, not while inactive: drawn (`ENHORSE_DRAW`), the post-draw
    /// (`EnHorse_PostDraw`), then a jump the update asked for (`postDrawFunc`).
    fn draw_update(&mut self, play: &mut PlayState) {
        if self.state_flags & ENHORSE_INACTIVE != 0 {
            return;
        }
        self.state_flags |= ENHORSE_DRAW;
        if self.ty == HORSE_HNI && self.state_flags & ENHORSE_FLAG_18 != 0 {
            // EnHorse_OverrideLimbDraw's gHorseIngoGerudoSaddleDL on limb 30: Phase 18.
            log::trace!("En_Horse: the Gerudo saddle isn't drawn (Phase 18)");
        }
        self.post_draw(play);
        match self.post_draw_func {
            Some(PostDraw::LowJumpInit) => self.low_jump_init(play),
            Some(PostDraw::HighJumpInit) => self.high_jump_init(play),
            Some(PostDraw::BridgeJumpInit) => self.bridge_jump_init(play),
            None => {}
        }
    }

    fn horse_jumping(&self) -> bool {
        self.state_flags & ENHORSE_JUMPING != 0
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        let eye = EYE_BLINK_INDICES[(self.blink_timer as usize).min(EYE_BLINK_INDICES.len() - 1)] as u32;
        let hidden = self.state_flags & ENHORSE_INACTIVE != 0;
        rs.switches = vec![eye, (self.state_flags & ENHORSE_JUMPING != 0) as u32, hidden as u32];
        rs
    }

    /// `EnHorse_Draw`: `Gfx_SetupDL_25Opa`, the skin (`func_800A6360`, `setTranslation` but while
    /// jumping), Epona's eyes by `EnHorse_OverrideLimbDraw` (limb 13).
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let Some(joints) = &rs.joints else { return };
        if rs.switches.get(2).copied().unwrap_or(0) != 0 {
            return;
        }
        let set_translation = rs.switches.get(rs::NO_TRANSLATION).copied().unwrap_or(0) == 0;
        let place = SkinPlace { scale: rs.scale, rot: rs.rot, pos: rs.pos, y_offset: rs.y_offset };
        let (transform, bones) = skin_draw(&self.skeleton, &joints.rot, &place, set_translation);
        let mut params = DrawParams::default();
        let bake = if self.ty == HORSE_EPONA {
            let eye = rs.switches.get(rs::EYES).copied().unwrap_or(0) as usize;
            if let Some(img) = self.eyes.get(eye.min(self.eyes.len().saturating_sub(1))) {
                params.texture_images = vec![SourceImage { source: SEG_EYES, image: img.clone() }];
            }
            BAKE_EPONA
        } else {
            BAKE_HNI
        };
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(bake)), transform, bones, params });
    }

    fn destroy(&mut self, play: &mut PlayState) {
        if self.state_flags & ENHORSE_DRAW != 0
            && let Some(me) = play.cur_actor
        {
            // Audio_StopSfxByPos(&this->unk_21C): its voice (here at the horse).
            play.audio.stop_sfx_by_pos(oot_game::audio::sfx::SfxPos::Actor(me));
        }
    }

    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        match id {
            0 => Some(ColliderMut::Cylinder(&mut self.collider_cylinder1)),
            1 => Some(ColliderMut::Cylinder(&mut self.collider_cylinder2)),
            2 => Some(ColliderMut::JntSph(&mut self.collider_jnt_sph)),
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

/// What Player reads of the horse it rides (`rideActor`), by its handle.
pub fn ridden(actors: &oot_game::actor_ctx::ActorContext, h: ActorHandle) -> Option<&EnHorse> {
    actors.get(h).and_then(|a| a.as_any().downcast_ref::<EnHorse>())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_stick_direction() {
        // Straight up: angle 0, magnitude to 60.
        assert_eq!(stick_direction(Vec2::new(0.0, 80.0)), (60.0, 0));
        // Right: -0x4000 (atan2(-x, y)).
        let (m, a) = stick_direction(Vec2::new(30.0, 0.0));
        assert_eq!(m, 30.0);
        assert_eq!(a, -0x4000);
    }

    #[test]
    fn the_cutscene_function_index() {
        assert_eq!(EnHorse::get_cutscene_function_index(36), 1);
        assert_eq!(EnHorse::get_cutscene_function_index(38), 3);
        assert_eq!(EnHorse::get_cutscene_function_index(65), 5);
        assert_eq!(EnHorse::get_cutscene_function_index(40), 0);
        assert_eq!(EnHorse::get_cutscene_function_index(1), 0);
    }

    #[test]
    fn ingos_animation_for_the_horses() {
        assert_eq!(EnHorse::set_ingo_animation(ENHORSE_ANIM_GALLOP, 5.0, false), (ENIN_HORSE_ANIM_SLOW_RIDE, 5.0));
        assert_eq!(EnHorse::set_ingo_animation(ENHORSE_ANIM_GALLOP, 5.0, true), (ENIN_HORSE_ANIM_LASHING_FAST, 5.0));
        assert_eq!(EnHorse::set_ingo_animation(ENHORSE_ANIM_WALK, 5.0, true), (ENIN_HORSE_ANIM_SITTING_FORWARD, 0.0));
        assert_eq!(EnHorse::set_ingo_animation(ENHORSE_ANIM_IDLE, 5.0, false), (ENIN_HORSE_ANIM_WHOOPING, 5.0));
    }
}
