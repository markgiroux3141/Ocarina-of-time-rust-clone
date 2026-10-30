//! `En_Mb` (`ovl_En_Mb/z_en_mb.c`): the Moblins (GAME-13 milestone 2). The adult's Sacred Forest
//! Meadow (`spot05`, layers 2 and 3) has five spear Moblins patrolling its maze's corridors (params
//! 0x00FF to 0x04FF, the scene's paths 0 to 4) and the club Moblin guarding the stairs to the
//! Forest Temple (params 0, at (-3, 160, -1357)); the Spirit Temple's room 13 has another club
//! Moblin.
//!
//! - **A spear patrol** (any params but -1 and 0; init writes 1, `ENMB_TYPE_SPEAR_PATROL`, over
//!   them): 0.014 big, 1 health. It walks its path to and fro at 0.6
//!   (`EnMb_SpearPatrolWalkTowardsWaypoint`), and at each point looks left and right 40 to 119
//!   frames, turning towards its home's facing, then turns to the next (0x3E8 a frame,
//!   `EnMb_SpearPatrolTurnTowardsWaypoint`). **It sees Link** (`EnMb_IsPlayerInCorridor`) within
//!   20 up or down, in a box 74 by 800 about it along its facing snapped to the nearest quarter
//!   turn (the meadow's corridors are 100 wide), and in front of it (a quarter turn either side),
//!   or anywhere in that box within 160 when he makes a noise (`func_8002DDE4`). It then picks the
//!   point of its path most nearly towards him (`EnMb_FindWaypointTowardsPlayer`), shouts, lowers
//!   its spear and **charges** at 10 (`EnMb_SpearPatrolPrepareAndCharge`) to the floor's end (no
//!   floor 110 ahead, `Actor_TestFloorInDirection`). **Its spear catches Link** (8 damage, then
//!   `play->grabPlayer`): he's carried 89 in front of it, and dropped (knocked down 4) at the
//!   charge's end. There it slides back up 6 and stops (`EnMb_SpearPatrolEndCharge`); 50 frames
//!   on it charges again if Link's in its corridor in front within 200, else stands its spear up
//!   and walks on. Hit, it's knocked 8 back (or on, hit from behind) and then charges along its
//!   path (`EnMb_SpearPatrolImmediateCharge`), turning at each end, until three ends are passed
//!   and it's within 80 of its home (`EnMb_SpearEndChargeQuick`).
//! - **The club Moblin** (params 0): 0.02 big, 6 health, its cylinder 45 by 170, not targetable.
//!   Placed facing away from Link (by more than a quarter turn) it turns round and moves 600 along
//!   z. Link within 710 of its home (not reeling from a hit, `PLAYER_STATE1_26`, and within 0x3E80
//!   of its facing), it **lifts its club** to the right, middle or left (by where Link is,
//!   0x258 either side the middle) and **brings it down** (`EnMb_ClubAttack`): the club itself
//!   hits for 8, unblockable, and knocks Link down by how close he is; where it lands, a white
//!   shockwave, dust, a camera quake (25, 5 frames) and **a shock wave along the ground**
//!   (`func_800358DC`: an `En_Part` of kind -1 at 18 a frame for 20 frames, which hurts Link 8 and
//!   knocks him down within 40). Hit, it kneels (`EnMb_ClubDamaged`, the quake again as it
//!   stands); at 0 it falls on its back (`EnMb_ClubDead`), shaking the ground at frames 15 and 22,
//!   then 30 frames of green-edged flames (`EffectSsDeadDb`) 70 behind it and a drop from table
//!   12.
//! - **A spear guard** (params -1, placed nowhere): 0.01 big, 2 health. It looks round
//!   (`EnMb_SpearGuardLookAround`) and walks about its home (`EnMb_SpearGuardWalk`, back within
//!   1000), turning to Link within 1750 of its home and charging him from within 500 in front;
//!   the charge ends as he's off its facing (0x1388).
//! - **Hit** (`EnMb_CheckColliding`; from `ENMB_STATE_STUNNED` on): the front of a spear Moblin
//!   (`frontShieldingCollider`, two hard triangles 8000 by 7000 at 3500 in front) takes a hit and
//!   nothing comes of it. The boomerang stuns a spear Moblin blue for 80 frames, the hookshot the
//!   club Moblin; the ice arrow and ice magic stun either, with ice (`EffectSsEnIce`) flying off
//!   ten of its limbs (`iceEffectTimer`); anything else hurts it (red, 12 frames), Link let go.
//!   A spear Moblin dies at 0 (`EnMb_SpearDead`: falls on its back, its flames, a drop from table
//!   14).
//!
//! The whole overlay is ported (every function above, `EnMb_SpearUpdateAttackCollider`,
//! `EnMb_ClubUpdateAttackCollider`, `EnMb_PostLimbDraw` and `EnMb_Draw` in `draw_update`: the
//! spear's quad and the front triangles from the actor's matrix, the club's quad from its left
//! hand, the club's feet for its shadow, the body parts for the ice). The version followed is
//! gc-eu-mq-dbg's (`OOT_VERSION >= NTSC_1_2`). Not ported: `Rumble_Request` (the controller's,
//! BACKLOG #60, logged). The draw is a skeleton bake each (`gEnMbSpearSkel`, `gEnMbClubSkel`) after
//! `Gfx_SetupDL_25Opa`. A Deku nut, fire or light magic gives `ENMB_DMG_REACT_FREEZE`, which
//! `EnMb_CheckColliding` passes over and which only skips `EnMb_Update`'s action and movement on
//! the frame of the hit (the reaction is the frame's).
//!
//! Params: `params` whole: -1 (0xFFFF) a spear guard, 0 the club Moblin, anything else a spear
//! patrol whose path is `PARAMS_GET_S(params, 8, 8)` (the meadow's 0x00FF to 0x04FF; the low byte
//! read by nothing). Init writes 1 (`ENMB_TYPE_SPEAR_PATROL`) over a patrol's params. The
//! rotation's z is its roll as placed.
//! Flags: none.

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, MeshKey};
use eng_math::{cos_s, sin_s, smooth_step_to_f, smooth_step_to_s, vec3f_yaw};
use glam::{Mat4, Vec3};
use oot_game::actor::*;
use oot_game::actor_ctx::{
    ACTORCAT_ENEMY, ActorImpl, ActorProfile, actor_set_player_knockback_large_no_damage, actor_spawn_floor_dust_ring, actor_test_floor_in_direction, audio_play_actor_sfx_at,
    audio_play_actor_sfx2, func_80033480,
};
use oot_game::actor_schema::{ActorSchema, Field, Kind, Source};
use oot_game::audio::sfx::{NA_SE_EN_GOMA_JR_FREEZE, NA_SE_PL_BODY_HIT};
use oot_game::camera::CAM_ID_MAIN;
use oot_game::collision_check::*;
use oot_game::pack::{BakeBody, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::skelanime_std::{ANIMMODE_LOOP, ANIMMODE_LOOP_INTERP, ANIMMODE_ONCE, ANIMMODE_ONCE_INTERP, Anim, LimbDraw, SkelAnimeStd, draw_opa_pose};

use crate::en_part::{ACTOR_EN_PART, EnPart};
use crate::player::{Player, STATE1_26, STATE2_3, STATE2_7};

/// `ACTOR_EN_MB` (`actor_table.h`: 0x004B).
pub const ACTOR_EN_MB: i16 = 0x004B;
/// `OBJECT_MB` (`object_table.h`: 0x0030).
pub const OBJECT: &str = "object_mb";
const SKEL_SPEAR: &str = "gEnMbSpearSkel";
const SKEL_CLUB: &str = "gEnMbClubSkel";

/// `En_Mb_Profile`: `ACTORCAT_ENEMY`, `FLAGS` (attention-enabled, hostile, always updating).
pub const PROFILE: ActorProfile =
    ActorProfile { id: ACTOR_EN_MB, name: "En_Mb", category: ACTORCAT_ENEMY, flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE | ACTOR_FLAG_UPDATE_CULLING_DISABLED, object: OBJECT, schema: &SCHEMA };

/// `this->actor.params`, switched on whole at init (`EnMbType`).
pub const PARAMS: Field = Field::whole(
    "params",
    Kind::Range(-0x8000, 0x7FFF),
    "the kind: -1 (0xFFFF) a spear guard round its home (placed nowhere), 0 the club Moblin, anything else a spear patrol on the path in bits 8 to 15",
);
/// `PARAMS_GET_S(thisx->params, 8, 8)` (a spear patrol's `path`).
pub const PATH: Field = Field::params("PARAMS_GET_S(params, 8, 8)", 8, 8, Kind::PathIndex, "a spear patrol's path (the scene layer's), walked to and fro").part_of("params");
/// The low byte: 0xFF in the patrols' data.
pub const PARAMS_LOW: Field = Field::params("params & 0xFF", 0, 8, Kind::Unused, "0xFF in the spear patrols' data; read by nothing").part_of("params");
/// `shape.rot.z`: one of the meadow's patrols is placed with -182.
pub const ROT_Z: Field = Field::rot("shape.rot.z", Source::RotZ, 0, 16, Kind::Rotation, "its roll as placed (drawn so)");

/// En_Mb's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema { fields: &[PARAMS, PATH, PARAMS_LOW, ROT_Z], flags: &[], todo: false, objects: &[], texts: &[] };

/// `EnMbType`.
pub const ENMB_TYPE_SPEAR_GUARD: i16 = -1;
pub const ENMB_TYPE_CLUB: i16 = 0;
pub const ENMB_TYPE_SPEAR_PATROL: i16 = 1;

/// `ENMB_ATTACK_*` (the spear's 1 is the club's right).
pub const ENMB_ATTACK_NONE: i16 = 0;
pub const ENMB_ATTACK_SPEAR: i16 = 1;
pub const ENMB_ATTACK_CLUB_RIGHT: i16 = 1;
pub const ENMB_ATTACK_CLUB_MIDDLE: i16 = 2;
pub const ENMB_ATTACK_CLUB_LEFT: i16 = 3;

/// `EnMbState` (`z_en_mb.h`).
pub const ENMB_STATE_SPEAR_SPEARPATH_DAMAGED: i32 = 0;
pub const ENMB_STATE_CLUB_DEAD: i32 = 1;
pub const ENMB_STATE_CLUB_KNEELING_DAMAGED: i32 = 2;
pub const ENMB_STATE_CLUB_KNEELING: i32 = 3;
pub const ENMB_STATE_STUNNED: i32 = 5;
pub const ENMB_STATE_IDLE: i32 = 6;
pub const ENMB_STATE_WALK: i32 = 9;
pub const ENMB_STATE_ATTACK: i32 = 10;
pub const ENMB_STATE_ATTACK_END: i32 = 11;

/// `EnMbLimb` (both skeletons' limbs are laid out alike; 1-based).
pub const ENMB_LIMB_ROOT: usize = 1;
pub const ENMB_LIMB_WAIST: usize = 3;
pub const ENMB_LIMB_CHEST: usize = 6;
pub const ENMB_LIMB_HEAD: usize = 7;
pub const ENMB_LIMB_LSHOULDER: usize = 9;
pub const ENMB_LIMB_LFOREARM: usize = 11;
pub const ENMB_LIMB_LHAND: usize = 12;
pub const ENMB_LIMB_RSHOULDER: usize = 14;
pub const ENMB_LIMB_RFOREARM: usize = 16;
pub const ENMB_LIMB_RHAND: usize = 17;
pub const ENMB_LIMB_LTHIGH: usize = 20;
pub const ENMB_LIMB_LSHIN: usize = 21;
pub const ENMB_LIMB_LFOOT: usize = 22;
pub const ENMB_LIMB_RTHIGH: usize = 25;
pub const ENMB_LIMB_RSHIN: usize = 26;
pub const ENMB_LIMB_RFOOT: usize = 27;
/// The skeletons' limbs (`SkelAnime_InitFlex`'s 28 joints: the root's position and 27 limbs).
const LIMB_COUNT: usize = 27;

/// `EnMbDamageReaction`.
pub const ENMB_DMG_REACT_IGNORE: u8 = 0x0;
pub const ENMB_DMG_REACT_STUN: u8 = 0x1;
pub const ENMB_DMG_REACT_FREEZE: u8 = 0x5;
pub const ENMB_DMG_REACT_STUN_ICE: u8 = 0x6;
pub const ENMB_DMG_REACT_DEFAULT: u8 = 0xF;

/// `NAVI_ENEMY_MOBLIN`, `NAVI_ENEMY_MOBLIN_CLUB` (`actor.h`: 0x4A, 0x4B).
pub const NAVI_ENEMY_MOBLIN: u8 = 0x4A;
pub const NAVI_ENEMY_MOBLIN_CLUB: u8 = 0x4B;

/// `COLLECTIBLE_DROP_TABLE_12`, `_14` (`z_en_item00.h`).
const COLLECTIBLE_DROP_TABLE_12: i16 = 12;
const COLLECTIBLE_DROP_TABLE_14: i16 = 14;

// The sounds (`include/tables/sfx/enemybank_table.h`).
/// `NA_SE_EN_MORIBLIN_WALK` (0x38B8).
pub const NA_SE_EN_MORIBLIN_WALK: u16 = 0x38B8;
/// `NA_SE_EN_MORIBLIN_SLIDE` (0x38B9).
pub const NA_SE_EN_MORIBLIN_SLIDE: u16 = 0x38B9;
/// `NA_SE_EN_MORIBLIN_ATTACK` (0x38BA).
pub const NA_SE_EN_MORIBLIN_ATTACK: u16 = 0x38BA;
/// `NA_SE_EN_MORIBLIN_VOICE` (0x38BB).
pub const NA_SE_EN_MORIBLIN_VOICE: u16 = 0x38BB;
/// `NA_SE_EN_MORIBLIN_SPEAR_AT` (0x38BC).
pub const NA_SE_EN_MORIBLIN_SPEAR_AT: u16 = 0x38BC;
/// `NA_SE_EN_MORIBLIN_SPEAR_NORM` (0x38BD).
pub const NA_SE_EN_MORIBLIN_SPEAR_NORM: u16 = 0x38BD;
/// `NA_SE_EN_MORIBLIN_DEAD` (0x38BE).
pub const NA_SE_EN_MORIBLIN_DEAD: u16 = 0x38BE;
/// `NA_SE_EN_MORIBLIN_DASH` (0x38BF).
pub const NA_SE_EN_MORIBLIN_DASH: u16 = 0x38BF;
/// `NA_SE_EN_MONBLIN_HAM_DOWN` (0x38E1).
pub const NA_SE_EN_MONBLIN_HAM_DOWN: u16 = 0x38E1;
/// `NA_SE_EN_MONBLIN_HAM_UP` (0x38E2).
pub const NA_SE_EN_MONBLIN_HAM_UP: u16 = 0x38E2;
/// `NA_SE_EN_MONBLIN_HAM_LAND` (0x39EF).
pub const NA_SE_EN_MONBLIN_HAM_LAND: u16 = 0x39EF;
/// `NA_SE_EN_RIZA_DOWN` (0x387A).
pub const NA_SE_EN_RIZA_DOWN: u16 = 0x387A;

/// The spear's walk at `0.59999996f` (as the C writes it).
const WALK_SPEED: f32 = 0.599_999_96;

/// The checked elements' damage (`0xFFCFFFFF`: everything Player does but the shield and the
/// Mirror Shield's beam).
const ELEM_AC: ColliderElementDamageInfoACInit = ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: HIT_BACKLASH_NONE, defense: 0 };
const ELEM_NO_AT: ColliderElementDamageInfoAT = ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0 };

/// `sBodyColliderInit`: 20 by 70, hit by Player, pushing everything.
const BODY_COLLIDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_HIT0, at_flags: AT_NONE, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_2, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit { elem_material: ELEM_MATERIAL_UNK1, at_dmg_info: ELEM_NO_AT, ac_dmg_info: ELEM_AC, at_elem_flags: ATELEM_NONE, ac_elem_flags: ACELEM_ON, oc_elem_flags: OCELEM_ON },
    dim: Cylinder16 { radius: 20, height: 70, y_shift: 0, pos: [0, 0, 0] },
};

/// An element of `sFrontShieldingTrisElementsInit`: hit by Player, hookable, no AT info.
const fn shield_element(vtx: [Vec3; 3]) -> ColliderTrisElementInit {
    ColliderTrisElementInit {
        info: ColliderElementInit {
            elem_material: ELEM_MATERIAL_UNK2,
            at_dmg_info: ELEM_NO_AT,
            ac_dmg_info: ELEM_AC,
            at_elem_flags: ATELEM_NONE,
            ac_elem_flags: ACELEM_ON | ACELEM_HOOKABLE | ACELEM_NO_AT_INFO,
            oc_elem_flags: OCELEM_NONE,
        },
        vtx,
    }
}

/// `sFrontShieldingTrisElementsInit`.
const FRONT_SHIELDING_ELEMENTS: [ColliderTrisElementInit; 2] = [
    shield_element([Vec3::new(-10.0, 14.0, 2.0), Vec3::new(-10.0, -6.0, 2.0), Vec3::new(9.0, 14.0, 2.0)]),
    shield_element([Vec3::new(-10.0, -6.0, 2.0), Vec3::new(9.0, -6.0, 2.0), Vec3::new(9.0, 14.0, 2.0)]),
];

/// `sFrontShieldingTrisInit`: metal, hit hard (a sword bounces off) by Player.
const FRONT_SHIELDING_INIT: ColliderInit =
    ColliderInit { col_type: COL_MATERIAL_METAL, at_flags: AT_NONE, ac_flags: AC_ON | AC_HARD | AC_TYPE_PLAYER, oc_flags1: OC1_NONE, oc_flags2: OC2_NONE, shape: COLSHAPE_TRIS };

/// `sAttackColliderQuadInit`: an enemy's attack (8, 0xFFCFFFFF; the club Moblin's unblockable).
const ATTACK_COLLIDER_INIT: ColliderQuadInit = ColliderQuadInit {
    base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_ON | AT_TYPE_ENEMY, ac_flags: AC_NONE, oc_flags1: OC1_NONE, oc_flags2: OC2_NONE, shape: COLSHAPE_QUAD },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0xFFCF_FFFF, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x08 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
        at_elem_flags: ATELEM_ON | ATELEM_SFX_NORMAL,
        ac_elem_flags: ACELEM_NONE,
        oc_elem_flags: OCELEM_NONE,
    },
    quad: [Vec3::ZERO; 4],
};

/// `sSpearMoblinDamageTable`.
pub static S_SPEAR_MOBLIN_DAMAGE_TABLE: DamageTable = DamageTable {
    table: [
        dmg_entry(0, ENMB_DMG_REACT_FREEZE),   // Deku nut
        dmg_entry(2, ENMB_DMG_REACT_DEFAULT),  // Deku stick
        dmg_entry(1, ENMB_DMG_REACT_DEFAULT),  // Slingshot
        dmg_entry(2, ENMB_DMG_REACT_DEFAULT),  // Explosive
        dmg_entry(0, ENMB_DMG_REACT_STUN),     // Boomerang
        dmg_entry(2, ENMB_DMG_REACT_DEFAULT),  // Normal arrow
        dmg_entry(2, ENMB_DMG_REACT_DEFAULT),  // Hammer swing
        dmg_entry(2, ENMB_DMG_REACT_DEFAULT),  // Hookshot
        dmg_entry(1, ENMB_DMG_REACT_DEFAULT),  // Kokiri sword
        dmg_entry(2, ENMB_DMG_REACT_DEFAULT),  // Master sword
        dmg_entry(4, ENMB_DMG_REACT_DEFAULT),  // Giant's Knife
        dmg_entry(2, ENMB_DMG_REACT_DEFAULT),  // Fire arrow
        dmg_entry(4, ENMB_DMG_REACT_STUN_ICE), // Ice arrow
        dmg_entry(2, ENMB_DMG_REACT_DEFAULT),  // Light arrow
        dmg_entry(4, ENMB_DMG_REACT_DEFAULT),  // Unk arrow 1
        dmg_entry(2, ENMB_DMG_REACT_DEFAULT),  // Unk arrow 2
        dmg_entry(2, ENMB_DMG_REACT_DEFAULT),  // Unk arrow 3
        dmg_entry(0, ENMB_DMG_REACT_FREEZE),   // Fire magic
        dmg_entry(3, ENMB_DMG_REACT_STUN_ICE), // Ice magic
        dmg_entry(0, ENMB_DMG_REACT_FREEZE),   // Light magic
        dmg_entry(0, ENMB_DMG_REACT_IGNORE),   // Shield
        dmg_entry(0, ENMB_DMG_REACT_IGNORE),   // Mirror Ray
        dmg_entry(1, ENMB_DMG_REACT_DEFAULT),  // Kokiri spin
        dmg_entry(4, ENMB_DMG_REACT_DEFAULT),  // Giant spin
        dmg_entry(2, ENMB_DMG_REACT_DEFAULT),  // Master spin
        dmg_entry(2, ENMB_DMG_REACT_DEFAULT),  // Kokiri jump
        dmg_entry(8, ENMB_DMG_REACT_DEFAULT),  // Giant jump
        dmg_entry(4, ENMB_DMG_REACT_DEFAULT),  // Master jump
        dmg_entry(0, ENMB_DMG_REACT_FREEZE),   // Unknown 1
        dmg_entry(0, ENMB_DMG_REACT_IGNORE),   // Unblockable
        dmg_entry(4, ENMB_DMG_REACT_DEFAULT),  // Hammer jump
        dmg_entry(0, ENMB_DMG_REACT_IGNORE),   // Unknown 2
    ],
};

/// `sClubMoblinDamageTable`: the spear's but the slingshot and the boomerang do nothing and the
/// hookshot stuns.
pub static S_CLUB_MOBLIN_DAMAGE_TABLE: DamageTable = DamageTable {
    table: [
        dmg_entry(0, ENMB_DMG_REACT_FREEZE),   // Deku nut
        dmg_entry(2, ENMB_DMG_REACT_DEFAULT),  // Deku stick
        dmg_entry(0, ENMB_DMG_REACT_IGNORE),   // Slingshot
        dmg_entry(2, ENMB_DMG_REACT_DEFAULT),  // Explosive
        dmg_entry(0, ENMB_DMG_REACT_IGNORE),   // Boomerang
        dmg_entry(2, ENMB_DMG_REACT_DEFAULT),  // Normal arrow
        dmg_entry(2, ENMB_DMG_REACT_DEFAULT),  // Hammer swing
        dmg_entry(0, ENMB_DMG_REACT_STUN),     // Hookshot
        dmg_entry(1, ENMB_DMG_REACT_DEFAULT),  // Kokiri sword
        dmg_entry(2, ENMB_DMG_REACT_DEFAULT),  // Master sword
        dmg_entry(4, ENMB_DMG_REACT_DEFAULT),  // Giant's Knife
        dmg_entry(2, ENMB_DMG_REACT_DEFAULT),  // Fire arrow
        dmg_entry(4, ENMB_DMG_REACT_STUN_ICE), // Ice arrow
        dmg_entry(2, ENMB_DMG_REACT_DEFAULT),  // Light arrow
        dmg_entry(4, ENMB_DMG_REACT_DEFAULT),  // Unk arrow 1
        dmg_entry(2, ENMB_DMG_REACT_DEFAULT),  // Unk arrow 2
        dmg_entry(2, ENMB_DMG_REACT_DEFAULT),  // Unk arrow 3
        dmg_entry(0, ENMB_DMG_REACT_FREEZE),   // Fire magic
        dmg_entry(3, ENMB_DMG_REACT_STUN_ICE), // Ice magic
        dmg_entry(0, ENMB_DMG_REACT_FREEZE),   // Light magic
        dmg_entry(0, ENMB_DMG_REACT_IGNORE),   // Shield
        dmg_entry(0, ENMB_DMG_REACT_IGNORE),   // Mirror Ray
        dmg_entry(1, ENMB_DMG_REACT_DEFAULT),  // Kokiri spin
        dmg_entry(4, ENMB_DMG_REACT_DEFAULT),  // Giant spin
        dmg_entry(2, ENMB_DMG_REACT_DEFAULT),  // Master spin
        dmg_entry(2, ENMB_DMG_REACT_DEFAULT),  // Kokiri jump
        dmg_entry(8, ENMB_DMG_REACT_DEFAULT),  // Giant jump
        dmg_entry(4, ENMB_DMG_REACT_DEFAULT),  // Master jump
        dmg_entry(0, ENMB_DMG_REACT_FREEZE),   // Unknown 1
        dmg_entry(0, ENMB_DMG_REACT_IGNORE),   // Unblockable
        dmg_entry(4, ENMB_DMG_REACT_DEFAULT),  // Hammer jump
        dmg_entry(0, ENMB_DMG_REACT_IGNORE),   // Unknown 2
    ],
};

/// The skeleton's bake (`gEnMbClubSkel` or `gEnMbSpearSkel` after `Gfx_SetupDL_25Opa`).
pub fn bake_name(club: bool) -> &'static str {
    if club { "En_Mb/club" } else { "En_Mb/spear" }
}

/// `EnMb_Draw`'s meshes: the two skeletons.
pub fn bakes() -> Vec<MeshBake> {
    [false, true]
        .into_iter()
        .map(|club| MeshBake {
            name: bake_name(club).into(),
            object: OBJECT.into(),
            segments: Vec::new(),
            prelude: Vec::new(),
            body: BakeBody::Skeleton { file: OBJECT.into(), symbol: if club { SKEL_CLUB } else { SKEL_SPEAR }.into(), limbs: Vec::new() },
        })
        .collect()
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Func {
    SpearGuardLookAround,
    SpearGuardWalk,
    SpearGuardPrepareAndCharge,
    SpearPatrolTurnTowardsWaypoint,
    SpearPatrolWalkTowardsWaypoint,
    SpearPatrolPrepareAndCharge,
    SpearPatrolImmediateCharge,
    SpearPatrolEndCharge,
    SpearEndChargeQuick,
    SpearDamaged,
    SpearDead,
    Stunned,
    ClubWaitPlayerNear,
    ClubAttack,
    ClubWaitAfterAttack,
    ClubDamaged,
    ClubDamagedWhileKneeling,
    ClubDead,
}

impl Func {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Func::SpearGuardLookAround => "EnMb_SpearGuardLookAround",
            Func::SpearGuardWalk => "EnMb_SpearGuardWalk",
            Func::SpearGuardPrepareAndCharge => "EnMb_SpearGuardPrepareAndCharge",
            Func::SpearPatrolTurnTowardsWaypoint => "EnMb_SpearPatrolTurnTowardsWaypoint",
            Func::SpearPatrolWalkTowardsWaypoint => "EnMb_SpearPatrolWalkTowardsWaypoint",
            Func::SpearPatrolPrepareAndCharge => "EnMb_SpearPatrolPrepareAndCharge",
            Func::SpearPatrolImmediateCharge => "EnMb_SpearPatrolImmediateCharge",
            Func::SpearPatrolEndCharge => "EnMb_SpearPatrolEndCharge",
            Func::SpearEndChargeQuick => "EnMb_SpearEndChargeQuick",
            Func::SpearDamaged => "EnMb_SpearDamaged",
            Func::SpearDead => "EnMb_SpearDead",
            Func::Stunned => "EnMb_Stunned",
            Func::ClubWaitPlayerNear => "EnMb_ClubWaitPlayerNear",
            Func::ClubAttack => "EnMb_ClubAttack",
            Func::ClubWaitAfterAttack => "EnMb_ClubWaitAfterAttack",
            Func::ClubDamaged => "EnMb_ClubDamaged",
            Func::ClubDamagedWhileKneeling => "EnMb_ClubDamagedWhileKneeling",
            Func::ClubDead => "EnMb_ClubDead",
        }
    }
}

/// The animations `object_mb` has that the overlay plays.
struct Anims {
    spear_damaged_from_behind: Anim,
    spear_fall_on_its_back: Anim,
    spear_damaged_from_front: Anim,
    spear_stand_still: Anim,
    spear_prepare_charge: Anim,
    spear_charge: Anim,
    spear_look_left_and_right: Anim,
    spear_slow_down: Anim,
    spear_walk: Anim,
    club_strike_down: Anim,
    club_lift_club: Anim,
    club_fall_on_its_back: Anim,
    club_damaged_kneel: Anim,
    club_beaten_kneeling: Anim,
    club_stand_up: Anim,
    club_stand_still_club_down: Anim,
}

impl Anims {
    fn load(play: &PlayState) -> Option<Anims> {
        let a = play.assets.clone()?;
        let get = |s: &str| a.animation(OBJECT, s).map_err(|e| log::error!("En_Mb: {e:#}")).ok();
        Some(Anims {
            spear_damaged_from_behind: get("gEnMbSpearDamagedFromBehindAnim")?,
            spear_fall_on_its_back: get("gEnMbSpearFallOnItsBackAnim")?,
            spear_damaged_from_front: get("gEnMbSpearDamagedFromFrontAnim")?,
            spear_stand_still: get("gEnMbSpearStandStillAnim")?,
            spear_prepare_charge: get("gEnMbSpearPrepareChargeAnim")?,
            spear_charge: get("gEnMbSpearChargeAnim")?,
            spear_look_left_and_right: get("gEnMbSpearLookLeftAndRightAnim")?,
            spear_slow_down: get("gEnMbSpearSlowDownAnim")?,
            spear_walk: get("gEnMbSpearWalkAnim")?,
            club_strike_down: get("gEnMbClubStrikeDownAnim")?,
            club_lift_club: get("gEnMbClubLiftClubAnim")?,
            club_fall_on_its_back: get("gEnMbClubFallOnItsBackAnim")?,
            club_damaged_kneel: get("gEnMbClubDamagedKneelAnim")?,
            club_beaten_kneeling: get("gEnMbClubBeatenKneelingAnim")?,
            club_stand_up: get("gEnMbClubStandUpAnim")?,
            club_stand_still_club_down: get("gEnMbClubStandStillClubDownAnim")?,
        })
    }
}

/// `ABS` of an s16 in an expression (an `int`: -0x8000 is 0x8000).
fn abs_i(v: i16) -> i32 {
    (v as i32).abs()
}

/// `ABS` of an s16 stored back in an s16 (-0x8000 stays -0x8000).
fn abs16(v: i16) -> i16 {
    (v as i32).abs() as i16
}

/// `Math_Vec3f_DistXZ`.
fn dist_xz(a: Vec3, b: Vec3) -> f32 {
    let (dx, dz) = (b.x - a.x, b.z - a.z);
    (dx * dx + dz * dz).sqrt()
}

/// `GET_PLAYER(play)`.
fn player(play: &PlayState) -> Option<&Player> {
    play.player.and_then(|h| play.actors.downcast::<Player>(h))
}

fn player_mut(play: &mut PlayState) -> Option<&mut Player> {
    let h = play.player?;
    play.actors.downcast_mut::<Player>(h)
}

// shared: func_8002DDE4
/// `func_8002DDE4` (`z_actor.c`): Player made a noise (`PLAYER_STATE2_3`).
fn func_8002dde4(play: &PlayState) -> bool {
    player(play).is_some_and(|p| p.state2 & STATE2_3 != 0)
}

/// Link held by this Moblin's spear (`PLAYER_STATE2_7` and his `parent` this actor).
fn player_held_by_me(play: &PlayState) -> bool {
    let me = play.cur_actor;
    player(play).is_some_and(|p| p.state2 & STATE2_7 != 0 && me.is_some() && p.actor.parent == me)
}

/// Link let go (`PLAYER_STATE2_7` off, no parent, `av2.actionVar2` 200 as `OOT_VERSION >=
/// PAL_1_0` writes it) and knocked down at `speed` along `yaw`, `vy` up
/// (`Actor_SetPlayerKnockbackLargeNoDamage`).
fn release_player(play: &mut PlayState, speed: f32, yaw: i16, vy: f32) {
    if let Some(p) = player_mut(play) {
        p.state2 &= !STATE2_7;
        p.actor.parent = None;
        p.action_var2 = 200;
    }
    actor_set_player_knockback_large_no_damage(play, speed, yaw, vy);
}

pub struct EnMb {
    pub actor: Actor,
    /// `bodyPartsPos`: ten limbs' places for the ice.
    pub body_parts_pos: [[i16; 3]; 10],
    /// `damageReaction`: the last hit's (`EnMb_SetupStunned` reads it for the ice).
    pub damage_reaction: u8,
    pub skel: SkelAnimeStd,
    skeleton: Option<Arc<Skeleton>>,
    anims: Option<Anims>,
    /// `state` (`EnMbState`).
    pub state: i32,
    pub func: Func,
    pub ice_effect_timer: i16,
    pub timer1: i16,
    pub timer2: i16,
    pub timer3: i16,
    pub yaw_to_waypoint: i16,
    /// `unk_332`: set to 1 by the walks, never read.
    pub unk_332: i16,
    /// `attack` (`ENMB_ATTACK_*`): above 0 its quad attacks.
    pub attack: i16,
    /// `effSpawnPos`: 8000 down the club Moblin's left hand (the club's head), from the draw.
    pub eff_spawn_pos: Vec3,
    pub waypoint_pos: Vec3,
    pub waypoint: i8,
    pub path: i8,
    pub direction: i8,
    pub max_home_dist: f32,
    pub player_detection_range: f32,
    pub body_collider: ColliderCylinder,
    /// `attackCollider`: the spear's or the club's quad.
    pub attack_collider: ColliderQuad,
    /// `frontShieldingCollider`.
    pub front_shielding_collider: ColliderTris,
}

impl EnMb {
    fn anim(&self, f: impl Fn(&Anims) -> &Anim) -> Option<Anim> {
        self.anims.as_ref().map(|a| f(a).clone())
    }

    /// `Animation_Change(anim, speed, start, end, mode, morph)`; `end` `None` its last frame.
    fn change(&mut self, f: impl Fn(&Anims) -> &Anim, speed: f32, start: f32, end: Option<f32>, mode: u8, morph: f32) {
        if let Some(a) = self.anim(f) {
            let end = end.unwrap_or_else(|| a.last_frame());
            self.skel.change(a, speed, start, end, mode, morph);
        }
    }

    /// `Animation_GetLastFrame`.
    fn last_frame(&self, f: impl Fn(&Anims) -> &Anim) -> f32 {
        self.anims.as_ref().map(|a| f(a).last_frame()).unwrap_or(0.0)
    }

    /// The type as the code reads it (`this->actor.params`).
    pub fn ty(&self) -> i16 {
        PARAMS.get(self.actor.params)
    }

    fn club(&self) -> bool {
        self.ty() == ENMB_TYPE_CLUB
    }

    /// `EnMb_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // sInitChain: naviEnemyId NAVI_ENEMY_MOBLIN, gravity -1, lockOnArrowOffset 5300.
        actor.navi_enemy_id = NAVI_ENEMY_MOBLIN;
        actor.gravity = -1.0;
        actor.target_arrow_offset = 5300.0;
        actor.shape_init(0.0, oot_game::shadow::ShadowDraw::Circle, 46.0);
        actor.col_chk_info.mass = MASS_IMMOVABLE;
        actor.col_chk_info.damage_table = Some(&S_SPEAR_MOBLIN_DAMAGE_TABLE);
        let ty = PARAMS.get(actor.params);
        let path = PATH.get(actor.params);
        let club = ty == ENMB_TYPE_CLUB;
        let skeleton = play.assets.clone().and_then(|a| a.skeleton(OBJECT, if club { SKEL_CLUB } else { SKEL_SPEAR }).map_err(|e| log::error!("En_Mb: {e:#}")).ok());
        let anims = Anims::load(play);
        let first = anims.as_ref().map(|a| if club { a.club_stand_still_club_down.clone() } else { a.spear_stand_still.clone() });
        let mut this = EnMb {
            actor,
            body_parts_pos: [[0; 3]; 10],
            damage_reaction: 0,
            // SkelAnime_InitFlex(.., gEnMbSpearStandStillAnim / gEnMbClubStandStillClubDownAnim, .., 28).
            skel: SkelAnimeStd::init_flex(LIMB_COUNT, first),
            skeleton,
            anims,
            state: ENMB_STATE_IDLE,
            func: Func::SpearGuardLookAround,
            ice_effect_timer: 0,
            timer1: 0,
            timer2: 0,
            timer3: 0,
            yaw_to_waypoint: 0,
            unk_332: 0,
            attack: ENMB_ATTACK_NONE,
            eff_spawn_pos: Vec3::ZERO,
            waypoint_pos: Vec3::ZERO,
            waypoint: 0,
            path: 0,
            direction: 0,
            max_home_dist: 0.0,
            player_detection_range: 0.0,
            body_collider: ColliderCylinder::new(&BODY_COLLIDER_INIT),
            attack_collider: ColliderQuad::new(&ATTACK_COLLIDER_INIT),
            front_shielding_collider: ColliderTris::new(&FRONT_SHIELDING_INIT, &FRONT_SHIELDING_ELEMENTS),
        };
        match ty {
            ENMB_TYPE_SPEAR_GUARD => {
                this.actor.col_chk_info.health = 2;
                this.actor.col_chk_info.mass = MASS_HEAVY;
                this.max_home_dist = 1000.0;
                this.player_detection_range = 1750.0;
                this.setup_spear_guard_look_around(play);
            }
            ENMB_TYPE_CLUB => {
                this.actor.col_chk_info.health = 6;
                this.actor.col_chk_info.mass = MASS_IMMOVABLE;
                this.actor.col_chk_info.damage_table = Some(&S_CLUB_MOBLIN_DAMAGE_TABLE);
                this.actor.scale = Vec3::splat(0.02);
                this.body_collider.dim.height = 170;
                this.body_collider.dim.radius = 45;
                this.actor.culling_volume_distance = 4000.0;
                this.actor.culling_volume_scale = 800.0;
                this.actor.culling_volume_downward = 1800.0;
                this.player_detection_range = 710.0;
                this.attack_collider.info.at_dmg_info.dmg_flags = DMG_UNBLOCKABLE;
                let player_pos = player(play).map(|p| p.actor.world_pos).unwrap_or(this.actor.world_pos);
                let rel_yaw_from_player = this.actor.world_rot.y.wrapping_sub(vec3f_yaw(this.actor.world_pos, player_pos));
                if abs_i(rel_yaw_from_player) > 0x4000 {
                    this.actor.world_rot.y = this.actor.world_rot.y.wrapping_add(-0x8000i32 as i16);
                    this.actor.shape_rot.y = this.actor.world_rot.y;
                    this.actor.world_pos.z += 600.0;
                }
                this.actor.shape_init(0.0, oot_game::shadow::ShadowDraw::Feet, 90.0);
                this.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
                this.actor.navi_enemy_id += NAVI_ENEMY_MOBLIN_CLUB - NAVI_ENEMY_MOBLIN;
                this.setup_club_wait_player_near(play);
            }
            _ => {
                // Spear Patrol.
                this.actor.scale = Vec3::splat(0.014);
                this.path = path as i8;
                // schema: init writes the type over the params
                this.actor.params = ENMB_TYPE_SPEAR_PATROL;
                this.waypoint = 0;
                this.actor.col_chk_info.health = 1;
                this.actor.col_chk_info.mass = MASS_HEAVY;
                this.max_home_dist = 350.0;
                this.player_detection_range = 1750.0;
                this.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
                this.setup_spear_patrol_turn_towards_waypoint(play);
            }
        }
        Box::new(this)
    }

    /// `EnMb_FaceWaypoint`.
    fn face_waypoint(&mut self) {
        let yaw_to_waypoint = vec3f_yaw(self.actor.world_pos, self.waypoint_pos);
        self.actor.shape_rot.y = yaw_to_waypoint;
        self.actor.world_rot.y = yaw_to_waypoint;
    }

    /// The path's point `i` (`SEGMENTED_TO_VIRTUAL(path->points)[i]`), and the path's count.
    fn path_point(&self, play: &PlayState, i: i32) -> Option<(Vec3, i32)> {
        let path = play.setup_path_list().get(self.path as u8 as usize)?;
        let p = path.points.get(i as usize)?;
        Some((Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32), path.count() as i32))
    }

    /// `EnMb_NextWaypoint`: on along the path, back at its ends.
    fn next_waypoint(&mut self, play: &PlayState) {
        let Some((_, count)) = self.path_point(play, 0) else {
            log::error!("En_Mb: path {} isn't the scene's (play->pathList[{}])", self.path, self.path);
            return;
        };
        if self.waypoint == 0 {
            self.direction = 1;
        } else if self.waypoint == (count - 1) as i8 {
            self.direction = -1;
        }
        self.waypoint = self.waypoint.wrapping_add(self.direction);
        if let Some((p, _)) = self.path_point(play, self.waypoint as i32) {
            self.waypoint_pos = p;
        }
    }

    /// `EnMb_IsPlayerInCorridor`: Link within 37 across and 400 along its facing, snapped to a
    /// quarter turn (`alignedYaw`).
    pub fn is_player_in_corridor(&self, play: &PlayState) -> bool {
        let Some(player_pos) = player(play).map(|p| p.actor.world_pos) else { return false };
        let rot_y = self.actor.world_rot.y;
        let aligned_yaw: i16 = if !(-0x62A2..=0x62A2).contains(&rot_y) {
            -0x8000
        } else if rot_y < -0x20E0 {
            -0x4000
        } else if rot_y > 0x20E0 {
            0x4000
        } else {
            0
        };
        let cos = cos_s(aligned_yaw).abs();
        let sin = sin_s(aligned_yaw).abs();
        let x_from_player = self.actor.world_pos.x - player_pos.x;
        let z_from_player = self.actor.world_pos.z - player_pos.z;
        x_from_player.abs() < cos * 37.0 + sin * 400.0 && z_from_player.abs() < sin * 37.0 + cos * 400.0
    }

    /// `EnMb_FindWaypointTowardsPlayer`: the first of the path's points (from 0) within 0x1770
    /// of the way to Link; facing it, the direction turned when it's the point it was making for.
    fn find_waypoint_towards_player(&mut self, play: &PlayState) {
        let Some((_, count)) = self.path_point(play, 0) else { return };
        for waypoint in 0..count {
            let Some((waypoint_pos, _)) = self.path_point(play, waypoint) else { break };
            let yaw_to_waypoint = vec3f_yaw(self.actor.world_pos, waypoint_pos);
            let yaw_player_to_waypoint = yaw_to_waypoint.wrapping_sub(self.actor.yaw_towards_player);
            if abs_i(yaw_player_to_waypoint) <= 0x1770 {
                self.actor.world_rot.y = yaw_to_waypoint;
                if waypoint == self.waypoint as i32 {
                    self.direction = self.direction.wrapping_neg();
                }
                self.waypoint_pos = waypoint_pos;
                self.waypoint = waypoint as i8;
                break;
            }
        }
    }

    /// `EnMb_SetupSpearGuardLookAround`.
    fn setup_spear_guard_look_around(&mut self, play: &mut PlayState) {
        // Animation_MorphToLoop(gEnMbSpearLookLeftAndRightAnim, -4).
        self.change(|a| &a.spear_look_left_and_right, 1.0, 0.0, None, ANIMMODE_LOOP, -4.0);
        self.actor.speed_xz = 0.0;
        self.timer1 = play.rand.s16_offset(30, 50);
        self.state = ENMB_STATE_IDLE;
        self.func = Func::SpearGuardLookAround;
    }

    /// `EnMb_SetupClubWaitPlayerNear`.
    fn setup_club_wait_player_near(&mut self, play: &mut PlayState) {
        // Animation_PlayLoop(gEnMbClubStandStillClubDownAnim).
        self.change(|a| &a.club_stand_still_club_down, 1.0, 0.0, None, ANIMMODE_LOOP, 0.0);
        self.actor.speed_xz = 0.0;
        self.timer1 = play.rand.s16_offset(30, 50);
        self.state = ENMB_STATE_IDLE;
        self.func = Func::ClubWaitPlayerNear;
    }

    /// `EnMb_SetupSpearPatrolTurnTowardsWaypoint`: looking round 40 to 119 frames, the next point
    /// picked.
    fn setup_spear_patrol_turn_towards_waypoint(&mut self, play: &mut PlayState) {
        self.change(|a| &a.spear_look_left_and_right, 1.0, 0.0, None, ANIMMODE_LOOP, -4.0);
        self.actor.speed_xz = 0.0;
        self.timer1 = play.rand.s16_offset(40, 80);
        self.state = ENMB_STATE_IDLE;
        self.next_waypoint(play);
        self.func = Func::SpearPatrolTurnTowardsWaypoint;
    }

    /// `EnMb_SetupSpearGuardWalk`.
    fn setup_spear_guard_walk(&mut self, play: &mut PlayState) {
        self.change(|a| &a.spear_walk, 0.0, 0.0, None, ANIMMODE_LOOP, -4.0);
        self.actor.speed_xz = WALK_SPEED;
        self.timer1 = play.rand.s16_offset(50, 70);
        self.unk_332 = 1;
        self.state = ENMB_STATE_WALK;
        self.func = Func::SpearGuardWalk;
    }

    /// `EnMb_SetupSpearPatrolWalkTowardsWaypoint`.
    fn setup_spear_patrol_walk_towards_waypoint(&mut self, play: &mut PlayState) {
        self.actor.speed_xz = WALK_SPEED;
        self.timer1 = play.rand.s16_offset(50, 70);
        self.unk_332 = 1;
        self.state = ENMB_STATE_WALK;
        self.change(|a| &a.spear_walk, 0.0, 0.0, None, ANIMMODE_LOOP_INTERP, -4.0);
        self.func = Func::SpearPatrolWalkTowardsWaypoint;
    }

    /// `EnMb_SetupSpearPrepareAndCharge`: the spear lowered (its last frame plus 6 to the charge).
    fn setup_spear_prepare_and_charge(&mut self, play: &mut PlayState) {
        let frame_count = self.last_frame(|a| &a.spear_prepare_charge);
        // Animation_MorphToPlayOnce(gEnMbSpearPrepareChargeAnim, -4).
        self.change(|a| &a.spear_prepare_charge, 1.0, 0.0, None, ANIMMODE_ONCE, -4.0);
        self.state = ENMB_STATE_ATTACK;
        self.actor.speed_xz = 0.0;
        self.timer3 = (frame_count as i16).wrapping_add(6);
        audio_play_actor_sfx2(play, NA_SE_EN_MORIBLIN_SPEAR_AT);
        self.func = if self.ty() == ENMB_TYPE_SPEAR_GUARD { Func::SpearGuardPrepareAndCharge } else { Func::SpearPatrolPrepareAndCharge };
    }

    /// `EnMb_SetupSpearPatrolImmediateCharge`.
    fn setup_spear_patrol_immediate_charge(&mut self, play: &mut PlayState) {
        // Animation_PlayLoop(gEnMbSpearChargeAnim).
        self.change(|a| &a.spear_charge, 1.0, 0.0, None, ANIMMODE_LOOP, 0.0);
        audio_play_actor_sfx2(play, NA_SE_EN_MORIBLIN_ATTACK);
        self.attack = ENMB_ATTACK_SPEAR;
        self.state = ENMB_STATE_ATTACK;
        self.timer3 = 3;
        self.actor.speed_xz = 10.0;
        self.func = Func::SpearPatrolImmediateCharge;
    }

    /// `EnMb_SetupClubAttack`: the club lifted at triple speed, to Link's side (0x258 either side
    /// the middle).
    fn setup_club_attack(&mut self) {
        self.state = ENMB_STATE_ATTACK;
        self.change(|a| &a.club_lift_club, 3.0, 0.0, None, ANIMMODE_ONCE_INTERP, 0.0);
        self.timer3 = 1;
        let rel_yaw_from_player = self.actor.world_rot.y.wrapping_sub(self.actor.yaw_towards_player);
        self.attack = if abs_i(rel_yaw_from_player) <= 0x258 {
            ENMB_ATTACK_CLUB_MIDDLE
        } else if rel_yaw_from_player >= 0 {
            ENMB_ATTACK_CLUB_RIGHT
        } else {
            ENMB_ATTACK_CLUB_LEFT
        };
        self.func = Func::ClubAttack;
    }

    /// `EnMb_SetupSpearEndChargeQuick`.
    fn setup_spear_end_charge_quick(&mut self, play: &mut PlayState) {
        // Animation_PlayOnce(gEnMbSpearSlowDownAnim).
        self.change(|a| &a.spear_slow_down, 1.0, 0.0, None, ANIMMODE_ONCE, 0.0);
        self.state = ENMB_STATE_ATTACK_END;
        self.timer1 = 0;
        self.timer3 = 5;
        audio_play_actor_sfx2(play, NA_SE_EN_MORIBLIN_SLIDE);
        self.func = Func::SpearEndChargeQuick;
    }

    /// `EnMb_SetupSpearPatrolEndCharge`: sliding back 8 and up 6.
    fn setup_spear_patrol_end_charge(&mut self, play: &mut PlayState) {
        self.change(|a| &a.spear_slow_down, 1.0, 0.0, None, ANIMMODE_ONCE, 0.0);
        self.state = ENMB_STATE_ATTACK_END;
        self.actor.bg_check_flags &= !BGCHECKFLAG_GROUND;
        self.timer1 = 0;
        self.timer3 = 50;
        self.actor.speed_xz = -8.0;
        self.actor.velocity.y = 6.0;
        audio_play_actor_sfx2(play, NA_SE_EN_MORIBLIN_SLIDE);
        self.func = Func::SpearPatrolEndCharge;
    }

    /// `EnMb_SetupClubWaitAfterAttack`.
    fn setup_club_wait_after_attack(&mut self) {
        self.state = ENMB_STATE_ATTACK_END;
        self.change(|a| &a.club_stand_still_club_down, 5.0, 0.0, None, ANIMMODE_ONCE_INTERP, 0.0);
        self.func = Func::ClubWaitAfterAttack;
    }

    /// `EnMb_SetupClubDamaged`.
    fn setup_club_damaged(&mut self, play: &mut PlayState) {
        self.change(|a| &a.club_damaged_kneel, 1.0, 0.0, None, ANIMMODE_ONCE, 0.0);
        self.state = ENMB_STATE_CLUB_KNEELING;
        self.timer1 = 0;
        self.timer3 = 20;
        audio_play_actor_sfx2(play, NA_SE_EN_MORIBLIN_DEAD);
        self.func = Func::ClubDamaged;
    }

    /// `EnMb_SetupClubDamagedWhileKneeling`.
    fn setup_club_damaged_while_kneeling(&mut self) {
        self.state = ENMB_STATE_CLUB_KNEELING_DAMAGED;
        self.timer1 = 0;
        self.timer3 = 6;
        self.change(|a| &a.club_beaten_kneeling, 1.0, 4.0, None, ANIMMODE_ONCE_INTERP, 0.0);
        self.func = Func::ClubDamagedWhileKneeling;
    }

    /// `EnMb_SetupClubDead`: on its back, its cylinder 95 by 80 (moved 4400 behind it by the
    /// update), not targetable.
    fn setup_club_dead(&mut self, play: &mut PlayState) {
        self.change(|a| &a.club_fall_on_its_back, 1.0, 0.0, None, ANIMMODE_ONCE, -4.0);
        self.state = ENMB_STATE_CLUB_DEAD;
        self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        self.body_collider.dim.height = 80;
        self.body_collider.dim.radius = 95;
        self.timer1 = 30;
        self.actor.speed_xz = 0.0;
        audio_play_actor_sfx2(play, NA_SE_EN_MORIBLIN_DEAD);
        self.func = Func::ClubDead;
    }

    /// `EnMb_SetupStunned`: blue for 80 frames; ice flying off for an ice hit, else a spear
    /// Moblin held at its first damaged frame.
    fn setup_stunned(&mut self, play: &mut PlayState) {
        self.state = ENMB_STATE_STUNNED;
        self.actor.speed_xz = 0.0;
        self.actor.set_color_filter(COLORFILTER_COLORFLAG_BLUE, 120, COLORFILTER_BUFFLAG_OPA, 80);
        if self.damage_reaction == ENMB_DMG_REACT_STUN_ICE {
            self.ice_effect_timer = 40;
        } else {
            if !self.club() {
                // Animation_PlayOnceSetSpeed(gEnMbSpearDamagedFromFrontAnim, 0).
                self.change(|a| &a.spear_damaged_from_front, 0.0, 0.0, None, ANIMMODE_ONCE, 0.0);
            }
            audio_play_actor_sfx2(play, NA_SE_EN_GOMA_JR_FREEZE);
        }
        self.func = Func::Stunned;
    }

    /// `EnMb_Stunned`: Link let go; the colour gone, dead, kneeling or waiting again.
    fn stunned(&mut self, play: &mut PlayState) {
        if player_held_by_me(play) {
            release_player(play, 4.0, self.actor.world_rot.y, 4.0);
            self.attack = ENMB_ATTACK_NONE;
        }
        if self.actor.color_filter_timer == 0 {
            if self.club() {
                if self.actor.col_chk_info.health == 0 {
                    self.setup_club_dead(play);
                } else if self.state == ENMB_STATE_CLUB_KNEELING {
                    // Dead code: EnMb_SetupStunned set the state to ENMB_STATE_STUNNED.
                    self.setup_club_damaged_while_kneeling();
                } else {
                    self.setup_club_wait_player_near(play);
                }
            } else if self.actor.col_chk_info.health == 0 {
                self.setup_spear_dead(play);
            } else {
                self.setup_spear_damaged(play);
            }
        }
    }

    /// `EnMb_SpearGuardLookAround`.
    fn spear_guard_look_around(&mut self, play: &mut PlayState) {
        self.skel.update();
        let timer1 = if self.timer1 == 0 {
            0
        } else {
            self.timer1 -= 1;
            self.timer1
        };
        if timer1 == 0 && self.skel.on_frame(0.0) {
            self.setup_spear_guard_walk(play);
        }
    }

    /// The spear Moblins' sight (`EnMb_SpearPatrolTurnTowardsWaypoint`,
    /// `EnMb_SpearPatrolWalkTowardsWaypoint`): Link within 20 up or down in its corridor, in front
    /// of it or (a noise) within 160: the charge. Whether it charges.
    fn spear_patrol_sees_player(&mut self, play: &mut PlayState, y_dist_to_player_abs: f32) -> bool {
        if y_dist_to_player_abs <= 20.0 && self.is_player_in_corridor(play) {
            let rel_yaw_from_player = self.actor.shape_rot.y.wrapping_sub(self.actor.yaw_towards_player);
            if abs_i(rel_yaw_from_player) <= 0x4000 || (func_8002dde4(play) && self.actor.xz_dist_to_player < 160.0) {
                self.find_waypoint_towards_player(play);
                audio_play_actor_sfx2(play, NA_SE_EN_MORIBLIN_VOICE);
                self.setup_spear_prepare_and_charge(play);
                return true;
            }
        }
        false
    }

    /// `EnMb_SpearPatrolTurnTowardsWaypoint`: looking round (towards its home's facing), then
    /// turning to its point (0x3E8 a frame) and walking.
    fn spear_patrol_turn_towards_waypoint(&mut self, play: &mut PlayState) {
        self.skel.update();
        if self.timer1 == 0 {
            self.yaw_to_waypoint = vec3f_yaw(self.actor.world_pos, self.waypoint_pos);
            if smooth_step_to_s(&mut self.actor.shape_rot.y, self.yaw_to_waypoint, 1, 0x3E8, 0) == 0 {
                self.actor.world_rot.y = self.actor.shape_rot.y;
                self.setup_spear_patrol_walk_towards_waypoint(play);
            }
        } else {
            self.timer1 -= 1;
            smooth_step_to_s(&mut self.actor.shape_rot.y, self.actor.home_rot.y, 1, 0x3E8, 0);
        }
        let y = self.actor.y_dist_to_player.abs();
        self.spear_patrol_sees_player(play, y);
    }

    /// The charge's dust (`Actor_SpawnFloorDustRing(play, &this->actor, &this->actor.world.pos,
    /// 5.0f, 3, 4.0f, 100, 15, false)`).
    fn dust(&self, play: &mut PlayState) {
        let pos = self.actor.world_pos;
        actor_spawn_floor_dust_ring(play, &self.actor, pos, 5.0, 3, 4.0, 100, 15, false);
    }

    /// The charge animation played backwards: the spear let go of, standing normally.
    fn spear_let_go(&mut self, play: &mut PlayState) {
        let last = self.last_frame(|a| &a.spear_prepare_charge);
        self.change(|a| &a.spear_prepare_charge, -1.0, last, Some(0.0), ANIMMODE_ONCE, 0.0);
        self.actor.speed_xz = 0.0;
        audio_play_actor_sfx2(play, NA_SE_EN_MORIBLIN_SPEAR_NORM);
    }

    /// `EnMb_SpearEndChargeQuick`: slowing down, the spear stood up, walking on.
    fn spear_end_charge_quick(&mut self, play: &mut PlayState) {
        smooth_step_to_f(&mut self.actor.speed_xz, 0.0, 0.5, 1.0, 0.0);
        if self.actor.speed_xz > 1.0 {
            self.dust(play);
        }
        if self.skel.update() {
            if self.timer1 == 0 {
                self.timer3 = self.timer3.wrapping_sub(1);
                if self.timer3 == 0 {
                    self.spear_let_go(play);
                    self.timer1 = 1;
                }
            } else if self.ty() <= ENMB_TYPE_SPEAR_GUARD {
                self.setup_spear_guard_walk(play);
                self.timer1 = 80;
                self.timer2 = 80;
                self.timer3 = 80;
            } else {
                self.setup_spear_patrol_turn_towards_waypoint(play);
            }
        }
    }

    /// `EnMb_ClubWaitAfterAttack`.
    fn club_wait_after_attack(&mut self, play: &mut PlayState) {
        self.attack = ENMB_ATTACK_NONE;
        if self.skel.update() {
            self.setup_club_wait_player_near(play);
        }
    }

    /// `EnMb_SpearPatrolEndCharge`: Link let go; on the ground, slowing, then charging again
    /// (Link in its corridor in front within 200) or the spear stood up and on along its path.
    fn spear_patrol_end_charge(&mut self, play: &mut PlayState) {
        if player_held_by_me(play) {
            release_player(play, 4.0, self.actor.world_rot.y, 4.0);
        }
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND == 0 {
            return;
        }
        smooth_step_to_f(&mut self.actor.speed_xz, 0.0, 1.0, 1.5, 0.0);
        if self.actor.speed_xz > 1.0 {
            self.dust(play);
        }
        if self.timer1 != 0 {
            self.timer3 = self.timer3.wrapping_sub(1);
            if self.timer3 == 0 {
                let rel_yaw_from_player = self.actor.shape_rot.y.wrapping_sub(self.actor.yaw_towards_player);
                if self.actor.y_dist_to_player.abs() <= 20.0
                    && self.is_player_in_corridor(play)
                    && abs_i(rel_yaw_from_player) <= 0x4000
                    && self.actor.xz_dist_to_player <= 200.0
                {
                    self.setup_spear_prepare_and_charge(play);
                } else {
                    self.spear_let_go(play);
                }
            } else if self.actor.xz_dist_to_player <= 160.0 {
                self.actor.speed_xz = -5.0;
            } else {
                self.actor.speed_xz = 0.0;
            }
        }
        if self.skel.update() {
            if self.timer1 == 0 {
                self.change(|a| &a.spear_charge, 0.5, 0.0, None, ANIMMODE_LOOP_INTERP, 0.0);
                self.timer1 = 1;
            } else {
                let yaw_player_to_waypoint = vec3f_yaw(self.actor.world_pos, self.waypoint_pos).wrapping_sub(self.actor.yaw_towards_player);
                if abs_i(yaw_player_to_waypoint) <= 0x4000 {
                    self.setup_spear_patrol_turn_towards_waypoint(play);
                } else {
                    self.setup_spear_patrol_walk_towards_waypoint(play);
                }
            }
        }
    }

    /// The dash's footsteps: frames 2 and 6 of the charge.
    fn dash_sfx(&self, play: &mut PlayState, prev_frame: i32) {
        let cur = self.skel.cur_frame as i32;
        if prev_frame != cur && (cur == 2 || cur == 6) {
            audio_play_actor_sfx2(play, NA_SE_EN_MORIBLIN_DASH);
        }
    }

    /// `EnMb_SpearGuardPrepareAndCharge`: turning to Link while the spear's lowered, then the
    /// charge, until he's off its facing (0x1388).
    fn spear_guard_prepare_and_charge(&mut self, play: &mut PlayState) {
        let rel_yaw_towards_player_abs = abs16(self.actor.yaw_towards_player.wrapping_sub(self.actor.shape_rot.y));
        let prev_frame = self.skel.cur_frame as i32;
        if self.skel.update() {
            self.change(|a| &a.spear_charge, 1.0, 0.0, None, ANIMMODE_LOOP, 0.0);
            audio_play_actor_sfx2(play, NA_SE_EN_MORIBLIN_ATTACK);
        }
        if self.timer3 != 0 {
            self.timer3 -= 1;
            smooth_step_to_s(&mut self.actor.world_rot.y, self.actor.yaw_towards_player, 1, 0xBB8, 0);
        } else {
            self.actor.speed_xz = 10.0;
            self.attack = ENMB_ATTACK_SPEAR;
            self.dust(play);
            self.dash_sfx(play, prev_frame);
        }
        if rel_yaw_towards_player_abs > 0x1388 {
            self.attack = ENMB_ATTACK_NONE;
            self.setup_spear_end_charge_quick(play);
        }
    }

    /// `play->damagePlayer(play, -8)` as the attacks write it: Link rolling (`invincibilityTimer`
    /// below 0) has it cleared, and is hurt unless it was -40 or less.
    fn hurt_rolling_player(play: &mut PlayState) {
        let Some(t) = player(play).map(|p| p.invincibility_timer) else { return };
        if t < 0 {
            if let Some(p) = player_mut(play) {
                p.invincibility_timer = 0;
            }
            if t > -40 {
                crate::player::play_damage_player(play, -8);
            }
        }
    }

    /// `EnMb_ClubAttack`: turning to the side picked; the club's hit knocks Link down (harder
    /// the closer); the club lifted, then brought down at 1.5: the shock wave.
    fn club_attack(&mut self, play: &mut PlayState) {
        let eff_white_shockwave_dynamics = Vec3::ZERO;
        let flames_params = [18.0f32, 18.0, 0.0];
        let rel_yaw_target: [i16; 3] = [-0x9C4, 0, 0xDAC];
        let target = rel_yaw_target.get((self.attack - 1) as usize).copied().unwrap_or(0).wrapping_add(self.actor.world_rot.y);
        smooth_step_to_s(&mut self.actor.shape_rot.y, target, 1, 0x2EE, 0);

        if self.attack_collider.base.at_flags & AT_HIT != 0 {
            self.attack_collider.base.at_flags &= !AT_HIT;
            if self.attack_collider.base.at.is_some() && self.attack_collider.base.at == play.player {
                let prev_player_invincibility_timer = player(play).map(|p| p.invincibility_timer).unwrap_or(0);
                Self::hurt_rolling_player(play);
                actor_set_player_knockback_large_no_damage(play, (650.0 - self.actor.xz_dist_to_player) * 0.04 + 4.0, self.actor.world_rot.y, 8.0);
                if let Some(p) = player_mut(play) {
                    p.invincibility_timer = prev_player_invincibility_timer;
                }
            }
        }

        if self.skel.update() {
            if self.timer3 != 0 {
                self.timer3 -= 1;
                if self.timer3 == 0 {
                    self.change(|a| &a.club_strike_down, 1.5, 0.0, None, ANIMMODE_ONCE_INTERP, 0.0);
                }
            } else {
                let mut eff_spawn_pos = self.eff_spawn_pos;
                eff_spawn_pos.y = self.actor.floor_height;
                audio_play_actor_sfx2(play, NA_SE_EN_MONBLIN_HAM_LAND);
                log::trace!("En_Mb: Rumble_Request(xzDistToPlayer, 255, 20, 150) not ported");
                play.with_ss(|s| s.blast_spawn_white_shockwave(eff_spawn_pos, eff_white_shockwave_dynamics, eff_white_shockwave_dynamics));
                func_80033480(play, eff_spawn_pos, 2.0, 3, 0x12C, 0xB4, 1);
                play.camera_request_quake(CAM_ID_MAIN, 25, 5);
                let rot = [self.actor.world_rot.x, self.actor.world_rot.y];
                func_800358dc(&mut self.actor, play, eff_spawn_pos, rot, flames_params, 20, -1);
                self.setup_club_wait_after_attack();
            }
        } else if self.timer3 != 0 && self.skel.cur_frame == 6.0 {
            audio_play_actor_sfx2(play, NA_SE_EN_MONBLIN_HAM_UP);
        } else if self.timer3 == 0 && self.skel.cur_frame == 3.0 {
            audio_play_actor_sfx2(play, NA_SE_EN_MONBLIN_HAM_DOWN);
        }
    }

    /// The spear's hit and hold (`EnMb_SpearPatrolPrepareAndCharge`,
    /// `EnMb_SpearPatrolImmediateCharge` alike): Link hit and not yet held, 8 off him (rolling)
    /// and caught (`play->grabPlayer`); held, carried 89 in front of it. Whether it hit or holds
    /// him.
    fn spear_hit_and_hold(&mut self, play: &mut PlayState, end_charge: bool) -> bool {
        let mut has_hit_player = false;
        if self.attack_collider.base.at_flags & AT_HIT != 0 {
            if self.attack_collider.base.at.is_some() && self.attack_collider.base.at == play.player {
                let held = player(play).is_some_and(|p| p.state2 & STATE2_7 != 0);
                if !end_charge && !held {
                    Self::hurt_rolling_player(play);
                    if self.attack_collider.base.at_flags & AT_BOUNCED == 0
                        && let Some(h) = play.player
                    {
                        audio_play_actor_sfx_at(play, h, NA_SE_PL_BODY_HIT);
                    }
                    if crate::player::play_grab_player(play) {
                        let me = play.cur_actor;
                        if let Some(p) = player_mut(play) {
                            p.actor.parent = me;
                        }
                    }
                }
                has_hit_player = true;
            } else {
                self.attack_collider.base.at_flags &= !AT_HIT;
            }
        }
        if player_held_by_me(play) {
            let (pos, rot_y) = (self.actor.world_pos, self.actor.shape_rot.y);
            if let Some(p) = player_mut(play) {
                p.actor.world_pos.x = pos.x + cos_s(rot_y) * 10.0 + sin_s(rot_y) * 89.0;
                p.actor.world_pos.z = pos.z + sin_s(rot_y) * 10.0 + cos_s(rot_y) * 89.0;
                p.action_var2 = 0;
                p.actor.speed_xz = 0.0;
                p.actor.velocity.y = 0.0;
            }
            has_hit_player = true;
        }
        has_hit_player
    }

    /// The charge's end with Link hit or held (`OOT_VERSION >= NTSC_1_2`): the hit forgotten,
    /// Link let go if anything holds him.
    fn spear_end_charge_release(&mut self, play: &mut PlayState) {
        self.attack_collider.base.at_flags &= !AT_HIT;
        // @bug (game): only PLAYER_STATE2_7 is checked, not that this Moblin holds him: Link held
        // by anything is let go as a charge ends (OOT_VERSION < NTSC_1_2 did the same unchecked).
        if player(play).is_some_and(|p| p.state2 & STATE2_7 != 0) {
            release_player(play, 4.0, self.actor.world_rot.y, 4.0);
        }
    }

    /// `EnMb_SpearPatrolPrepareAndCharge`: the spear lowered, turning to the charge's way (0x1F40
    /// a frame), then the charge at 10 to the floor's end (none 110 ahead).
    fn spear_patrol_prepare_and_charge(&mut self, play: &mut PlayState) {
        let rot_y = self.actor.world_rot.y;
        let mut end_charge = actor_test_floor_in_direction(&mut self.actor, play, 110.0, rot_y) == 0;
        let prev_frame = self.skel.cur_frame as i32;
        if self.skel.update() {
            self.change(|a| &a.spear_charge, 1.0, 0.0, None, ANIMMODE_LOOP, 0.0);
            audio_play_actor_sfx2(play, NA_SE_EN_MORIBLIN_ATTACK);
        }
        if self.timer3 != 0 {
            self.timer3 -= 1;
            smooth_step_to_s(&mut self.actor.shape_rot.y, self.actor.world_rot.y, 1, 0x1F40, 0);
            end_charge = false;
        } else {
            self.actor.speed_xz = 10.0;
            self.attack = ENMB_ATTACK_SPEAR;
            self.dust(play);
            self.dash_sfx(play, prev_frame);
        }
        let has_hit_player = self.spear_hit_and_hold(play, end_charge);
        if end_charge {
            if has_hit_player || player(play).is_some_and(|p| p.state2 & STATE2_7 != 0) {
                self.spear_end_charge_release(play);
            }
            self.attack = ENMB_ATTACK_NONE;
            self.actor.speed_xz = -10.0;
            self.setup_spear_patrol_end_charge(play);
        }
    }

    /// `EnMb_SpearPatrolImmediateCharge`: along the path at 10, turning at each end, until Link's
    /// hit (the charge's end) or, three ends on, it's within 80 of its home.
    fn spear_patrol_immediate_charge(&mut self, play: &mut PlayState) {
        let rot_y = self.actor.world_rot.y;
        let end_charge = actor_test_floor_in_direction(&mut self.actor, play, 110.0, rot_y) == 0;
        let prev_frame = self.skel.cur_frame as i32;
        self.skel.update();
        self.dust(play);
        self.dash_sfx(play, prev_frame);
        let has_hit_player = self.spear_hit_and_hold(play, end_charge);
        if end_charge {
            if has_hit_player || player(play).is_some_and(|p| p.state2 & STATE2_7 != 0) {
                self.spear_end_charge_release(play);
                self.attack = ENMB_ATTACK_NONE;
                self.actor.speed_xz = -10.0;
                self.setup_spear_patrol_end_charge(play);
                self.timer3 = 1;
            } else {
                self.timer3 = self.timer3.wrapping_sub(1);
                self.next_waypoint(play);
            }
        }
        self.face_waypoint();
        self.actor.shape_rot.y = self.actor.world_rot.y;
        if self.timer3 == 0 && dist_xz(self.actor.home_pos, self.actor.world_pos) < 80.0 {
            self.attack = ENMB_ATTACK_NONE;
            self.setup_spear_end_charge_quick(play);
        }
    }

    /// `EnMb_ClubDamaged`: kneeling, then standing up (the quake), then waiting again.
    fn club_damaged(&mut self, play: &mut PlayState) {
        if self.skel.update() {
            if self.timer3 != 0 {
                self.change(|a| &a.club_stand_up, 1.0, 0.0, None, ANIMMODE_ONCE, 0.0);
                self.timer3 = 0;
                log::trace!("En_Mb: Rumble_Request(xzDistToPlayer, 255, 20, 150) not ported");
                play.camera_request_quake(CAM_ID_MAIN, 25, 5);
            } else {
                self.setup_club_wait_player_near(play);
            }
        }
    }

    /// `EnMb_ClubDamagedWhileKneeling` (reached only from `EnMb_Stunned`'s dead code).
    fn club_damaged_while_kneeling(&mut self, play: &mut PlayState) {
        if self.skel.update() {
            if self.timer3 != 0 {
                self.timer3 -= 1;
                if self.timer3 == 0 {
                    self.change(|a| &a.club_stand_up, 3.0, 0.0, None, ANIMMODE_ONCE_INTERP, 0.0);
                    if self.timer1 == 0 {
                        self.timer1 = 1;
                        self.timer3 = 6;
                    }
                }
            } else {
                self.setup_club_wait_player_near(play);
            }
        }
    }

    /// `EnMb_ClubDead`: falling on its back (the ground shaken at frames 15 and 22), then 30
    /// frames of flames (`EffectSsDeadDb`, white edged green) 70 behind it, then a drop from
    /// table 12 there.
    fn club_dead(&mut self, play: &mut PlayState) {
        let mut eff_pos = self.actor.world_pos;
        eff_pos.x += sin_s(self.actor.shape_rot.y) * -70.0;
        eff_pos.z += cos_s(self.actor.shape_rot.y) * -70.0;
        smooth_step_to_f(&mut self.actor.speed_xz, 0.0, 1.0, 0.5, 0.0);
        let eff_pos_base = eff_pos;
        if self.skel.update() {
            if self.timer1 > 0 {
                let eff_zero_vec = Vec3::ZERO;
                self.timer1 -= 1;
                for _ in 0..5 {
                    eff_pos.x = play.rand.centered_float(240.0) + eff_pos_base.x;
                    eff_pos.y = play.rand.centered_float(15.0) + (eff_pos_base.y + 20.0);
                    eff_pos.z = play.rand.centered_float(240.0) + eff_pos_base.z;
                    play.with_ss(|s| s.dead_db_spawn(eff_pos, eff_zero_vec, eff_zero_vec, 230, 7, [255, 255, 255, 255], [0, 255, 0], 1, 9, 1));
                }
            } else {
                crate::en_item00::item_drop_collectible_random(play, Some(&self.actor), eff_pos, COLLECTIBLE_DROP_TABLE_12 * 16);
                self.actor.kill();
            }
        } else if self.skel.cur_frame as i32 == 15 || self.skel.cur_frame as i32 == 22 {
            log::trace!("En_Mb: Rumble_Request(xzDistToPlayer, 255, 20, 150) not ported");
            actor_spawn_floor_dust_ring(play, &self.actor, eff_pos, 50.0, 10, 3.0, 400, 60, false);
            audio_play_actor_sfx2(play, NA_SE_EN_RIZA_DOWN);
            play.camera_request_quake(CAM_ID_MAIN, 25, 5);
        }
    }

    /// The walks' footsteps (`EnMb_SpearGuardWalk`, `EnMb_SpearPatrolWalkTowardsWaypoint`): the
    /// animation's frame passed 1 or 20 this update.
    fn walk_sfx(&self, play: &mut PlayState, prev_frame: i32, before_cur_frame: i32, play_speed_abs: f32) {
        if prev_frame != self.skel.cur_frame as i32 {
            let s = play_speed_abs as i32;
            if (before_cur_frame <= 1 && prev_frame + s >= 1) || (before_cur_frame <= 20 && prev_frame + s >= 20) {
                audio_play_actor_sfx2(play, NA_SE_EN_MORIBLIN_WALK);
            }
        }
    }

    /// `EnMb_SpearGuardWalk`: round its home (back within `maxHomeDist`), turning to Link within
    /// `playerDetectionRange` of its home and charging from within 500 in front.
    fn spear_guard_walk(&mut self, play: &mut PlayState) {
        let player_pos = player(play).map(|p| p.actor.world_pos).unwrap_or(Vec3::splat(f32::MAX));
        let rel_yaw_towards_player = abs16(self.actor.yaw_towards_player.wrapping_sub(self.actor.shape_rot.y));
        smooth_step_to_f(&mut self.actor.speed_xz, WALK_SPEED, 0.1, 1.0, 0.0);
        self.skel.play_speed = self.actor.speed_xz;
        let prev_frame = self.skel.cur_frame as i32;
        self.skel.update();
        let play_speed_abs = self.skel.play_speed.abs();
        let before_cur_frame = (self.skel.cur_frame - play_speed_abs) as i32;
        if self.timer3 == 0 && dist_xz(self.actor.home_pos, player_pos) < self.player_detection_range {
            smooth_step_to_s(&mut self.actor.world_rot.y, self.actor.yaw_towards_player, 1, 0x2EE, 0);
            self.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED;
            if self.actor.xz_dist_to_player < 500.0 && rel_yaw_towards_player < 0x1388 {
                self.setup_spear_prepare_and_charge(play);
            }
        } else {
            self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
            if dist_xz(self.actor.world_pos, self.actor.home_pos) > self.max_home_dist || self.timer2 != 0 {
                let yaw_towards_home = vec3f_yaw(self.actor.world_pos, self.actor.home_pos);
                smooth_step_to_s(&mut self.actor.world_rot.y, yaw_towards_home, 1, 0x2EE, 0);
            }
            if self.timer2 != 0 {
                self.timer2 -= 1;
            }
            if self.timer3 != 0 {
                self.timer3 -= 1;
            }
            if self.timer2 == 0 {
                audio_play_actor_sfx2(play, NA_SE_EN_MORIBLIN_VOICE);
            }
            self.timer1 = self.timer1.wrapping_sub(1);
            if self.timer1 == 0 {
                if play.rand.zero_one() > 0.7 {
                    self.timer1 = play.rand.s16_offset(50, 70);
                    self.timer2 = play.rand.s16_offset(15, 40);
                } else {
                    self.setup_spear_guard_look_around(play);
                }
            }
        }
        self.walk_sfx(play, prev_frame, before_cur_frame, play_speed_abs);
        self.actor.shape_rot.y = self.actor.world_rot.y;
    }

    /// `EnMb_SpearPatrolWalkTowardsWaypoint`: to its point (0x5DC a frame), the animation at
    /// twice its speed; looking round at the point (or now and then over its home); shouting
    /// every 30 to 99 frames.
    fn spear_patrol_walk_towards_waypoint(&mut self, play: &mut PlayState) {
        if dist_xz(self.waypoint_pos, self.actor.world_pos) <= 8.0 || (play.rand.zero_one() < 0.1 && dist_xz(self.actor.home_pos, self.actor.world_pos) <= 4.0) {
            self.setup_spear_patrol_turn_towards_waypoint(play);
        } else {
            smooth_step_to_f(&mut self.actor.speed_xz, WALK_SPEED, 0.1, 1.0, 0.0);
            self.skel.play_speed = 2.0 * self.actor.speed_xz;
        }
        self.yaw_to_waypoint = vec3f_yaw(self.actor.world_pos, self.waypoint_pos);
        smooth_step_to_s(&mut self.actor.world_rot.y, self.yaw_to_waypoint, 1, 0x5DC, 0);
        let y = self.actor.y_dist_to_player.abs();
        if self.spear_patrol_sees_player(play, y) {
            return;
        }
        if self.timer2 != 0 {
            self.timer2 -= 1;
        }
        if self.timer3 != 0 {
            self.timer3 -= 1;
        }
        if self.timer2 == 0 {
            audio_play_actor_sfx2(play, NA_SE_EN_MORIBLIN_VOICE);
            self.timer2 = play.rand.s16_offset(30, 70);
        }
        let prev_frame = self.skel.cur_frame as i32;
        self.skel.update();
        let play_speed_abs = self.skel.play_speed.abs();
        let before_cur_frame = (self.skel.cur_frame - play_speed_abs) as i32;
        self.walk_sfx(play, prev_frame, before_cur_frame, play_speed_abs);
        self.actor.shape_rot.y = self.actor.world_rot.y;
    }

    /// `EnMb_ClubWaitPlayerNear`: Link within `playerDetectionRange` (710) of its home, not reeling
    /// from a hit (`PLAYER_STATE1_26`), within 0x3E80 of its facing: the club lifted.
    fn club_wait_player_near(&mut self, play: &mut PlayState) {
        let (player_pos, state1) = player(play).map(|p| (p.actor.world_pos, p.state1)).unwrap_or((Vec3::splat(f32::MAX), 0));
        let rel_yaw_from_player = self.actor.world_rot.y.wrapping_sub(self.actor.yaw_towards_player);
        self.skel.update();
        if dist_xz(self.actor.home_pos, player_pos) < self.player_detection_range && state1 & STATE1_26 == 0 && abs_i(rel_yaw_from_player) < 0x3E80 {
            self.setup_club_attack();
        }
    }

    /// `EnMb_SetupSpearDamaged`: knocked 8 back (hit from in front) or on.
    fn setup_spear_damaged(&mut self, play: &mut PlayState) {
        let rel_yaw_towards_player = self.actor.yaw_towards_player.wrapping_sub(self.actor.shape_rot.y);
        if abs_i(rel_yaw_towards_player) <= 0x4000 {
            self.change(|a| &a.spear_damaged_from_front, 1.0, 0.0, None, ANIMMODE_ONCE, -4.0);
            self.actor.speed_xz = -8.0;
        } else {
            self.change(|a| &a.spear_damaged_from_behind, 1.0, 0.0, None, ANIMMODE_ONCE, -4.0);
            self.actor.speed_xz = 8.0;
        }
        self.timer1 = 30;
        self.state = ENMB_STATE_SPEAR_SPEARPATH_DAMAGED;
        self.actor.shape_rot.y = self.actor.world_rot.y;
        audio_play_actor_sfx2(play, NA_SE_EN_MORIBLIN_DEAD);
        self.func = Func::SpearDamaged;
    }

    /// `EnMb_SpearDamaged`: then a guard looks round, a patrol charges along its path.
    fn spear_damaged(&mut self, play: &mut PlayState) {
        smooth_step_to_f(&mut self.actor.speed_xz, 0.0, 1.0, 0.5, 0.0);
        if self.skel.update() {
            if self.ty() <= ENMB_TYPE_SPEAR_GUARD {
                self.setup_spear_guard_look_around(play);
            } else {
                self.setup_spear_patrol_immediate_charge(play);
            }
        }
    }

    /// `EnMb_SetupSpearDead`: knocked 8 back or on, not targetable.
    fn setup_spear_dead(&mut self, play: &mut PlayState) {
        let rel_yaw_towards_player = self.actor.yaw_towards_player.wrapping_sub(self.actor.shape_rot.y);
        self.change(|a| &a.spear_fall_on_its_back, 1.0, 0.0, None, ANIMMODE_ONCE, -4.0);
        if abs_i(rel_yaw_towards_player) <= 0x4000 {
            self.actor.speed_xz = -8.0;
        } else {
            // @bug (game): hit from behind it falls on its back all the same (forwards);
            // gEnMbSpearFallFaceDownAnim, unused, was probably meant here.
            self.actor.speed_xz = 8.0;
        }
        self.actor.world_rot.y = self.actor.shape_rot.y;
        self.timer1 = 30;
        self.state = ENMB_STATE_SPEAR_SPEARPATH_DAMAGED;
        audio_play_actor_sfx2(play, NA_SE_EN_MORIBLIN_DEAD);
        self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        self.func = Func::SpearDead;
    }

    /// `EnMb_SpearDead`: Link let go; fallen, 30 frames of flames about it (its shadow
    /// gone), then a drop from table 14.
    fn spear_dead(&mut self, play: &mut PlayState) {
        smooth_step_to_f(&mut self.actor.speed_xz, 0.0, 1.0, 0.5, 0.0);
        if player_held_by_me(play) {
            release_player(play, 4.0, self.actor.world_rot.y, 4.0);
            self.attack = ENMB_ATTACK_NONE;
        }
        if self.skel.update() {
            if self.timer1 > 0 {
                let zero_vec = Vec3::ZERO;
                self.actor.shape_shadow_scale = 0.0;
                self.timer1 -= 1;
                for _ in 0..5 {
                    let x = play.rand.centered_float(110.0) + self.actor.world_pos.x;
                    let y = play.rand.centered_float(15.0) + (self.actor.world_pos.y + 20.0);
                    let z = play.rand.centered_float(110.0) + self.actor.world_pos.z;
                    play.with_ss(|s| s.dead_db_spawn(Vec3::new(x, y, z), zero_vec, zero_vec, 100, 7, [255, 255, 255, 255], [0, 255, 0], 1, 9, 1));
                }
            } else {
                let pos = self.actor.world_pos;
                crate::en_item00::item_drop_collectible_random(play, Some(&self.actor), pos, COLLECTIBLE_DROP_TABLE_14 * 16);
                self.actor.kill();
            }
        }
    }

    /// `EnMb_CheckColliding`: a hit on its front comes to nothing; one on its body (stunned or
    /// on) stuns it or hurts it.
    fn check_colliding(&mut self, play: &mut PlayState) {
        if self.front_shielding_collider.base.ac_flags & AC_HIT != 0 {
            self.front_shielding_collider.base.ac_flags &= !(AC_HIT | AC_BOUNCED);
            self.body_collider.base.ac_flags &= !AC_HIT;
            return;
        }
        if !(self.body_collider.base.ac_flags & AC_HIT != 0 && self.state >= ENMB_STATE_STUNNED) {
            return;
        }
        self.body_collider.base.ac_flags &= !AC_HIT;
        let reaction = self.actor.col_chk_info.damage_reaction;
        if reaction == ENMB_DMG_REACT_IGNORE || reaction == ENMB_DMG_REACT_FREEZE {
            return;
        }
        if player_held_by_me(play) {
            release_player(play, 6.0, self.actor.world_rot.y, 6.0);
        }
        self.damage_reaction = reaction;
        self.attack = ENMB_ATTACK_NONE;
        let elem = self.body_collider.info;
        self.actor.set_drop_flag(&elem, false);
        if reaction == ENMB_DMG_REACT_STUN || reaction == ENMB_DMG_REACT_STUN_ICE {
            if self.state != ENMB_STATE_STUNNED {
                self.actor.apply_damage();
                self.setup_stunned(play);
            }
        } else {
            self.actor.apply_damage();
            self.actor.set_color_filter(COLORFILTER_COLORFLAG_RED, 250, COLORFILTER_BUFFLAG_OPA, 12);
            if self.club() {
                if self.actor.col_chk_info.health == 0 {
                    self.setup_club_dead(play);
                } else if self.state != ENMB_STATE_CLUB_KNEELING {
                    self.setup_club_damaged(play);
                }
            } else if self.actor.col_chk_info.health == 0 {
                self.setup_spear_dead(play);
            } else {
                self.setup_spear_damaged(play);
            }
        }
    }

    fn run_action(&mut self, play: &mut PlayState) {
        match self.func {
            Func::SpearGuardLookAround => self.spear_guard_look_around(play),
            Func::SpearGuardWalk => self.spear_guard_walk(play),
            Func::SpearGuardPrepareAndCharge => self.spear_guard_prepare_and_charge(play),
            Func::SpearPatrolTurnTowardsWaypoint => self.spear_patrol_turn_towards_waypoint(play),
            Func::SpearPatrolWalkTowardsWaypoint => self.spear_patrol_walk_towards_waypoint(play),
            Func::SpearPatrolPrepareAndCharge => self.spear_patrol_prepare_and_charge(play),
            Func::SpearPatrolImmediateCharge => self.spear_patrol_immediate_charge(play),
            Func::SpearPatrolEndCharge => self.spear_patrol_end_charge(play),
            Func::SpearEndChargeQuick => self.spear_end_charge_quick(play),
            Func::SpearDamaged => self.spear_damaged(play),
            Func::SpearDead => self.spear_dead(play),
            Func::Stunned => self.stunned(play),
            Func::ClubWaitPlayerNear => self.club_wait_player_near(play),
            Func::ClubAttack => self.club_attack(play),
            Func::ClubWaitAfterAttack => self.club_wait_after_attack(play),
            Func::ClubDamaged => self.club_damaged(play),
            Func::ClubDamagedWhileKneeling => self.club_damaged_while_kneeling(play),
            Func::ClubDead => self.club_dead(play),
        }
    }

    /// `EnMb_SpearUpdateAttackCollider`: the spear's quad from the actor's matrix (`mtx`): 2000
    /// wide, 1500 up, 0 to 4500 ahead; a patrol's 6000 wide, -4000 to 8500.
    fn spear_update_attack_collider(&mut self, mtx: &Mat4) {
        let mut quad_model0 = Vec3::new(1000.0, 1500.0, 0.0);
        let mut quad_model1 = Vec3::new(-1000.0, 1500.0, 0.0);
        let mut quad_model2 = Vec3::new(1000.0, 1500.0, 4500.0);
        let mut quad_model3 = Vec3::new(-1000.0, 1500.0, 4500.0);
        if self.ty() >= ENMB_TYPE_SPEAR_PATROL {
            quad_model0.x += 2000.0;
            quad_model0.z = -4000.0;
            quad_model1.z = -4000.0;
            quad_model2.x += 2000.0;
            quad_model1.x -= 2000.0;
            quad_model3.x -= 2000.0;
            quad_model2.z += 4000.0;
            quad_model3.z += 4000.0;
        }
        let q1 = mtx.transform_point3(quad_model0);
        let q0 = mtx.transform_point3(quad_model1);
        let q3 = mtx.transform_point3(quad_model2);
        let q2 = mtx.transform_point3(quad_model3);
        self.attack_collider.set_vertices(q0, q1, q2, q3);
    }

    /// `EnMb_ClubUpdateAttackCollider`: the club's quad from the left hand's matrix.
    fn club_update_attack_collider(&mut self, mtx: &Mat4) {
        const QUAD_MODEL: [Vec3; 4] = [Vec3::new(1000.0, 0.0, 0.0), Vec3::new(1000.0, 0.0, 0.0), Vec3::new(1000.0, -8000.0, -1500.0), Vec3::new(1000.0, -9000.0, 2000.0)];
        let q1 = mtx.transform_point3(QUAD_MODEL[0]);
        let q0 = mtx.transform_point3(QUAD_MODEL[1]);
        let q3 = mtx.transform_point3(QUAD_MODEL[2]);
        let q2 = mtx.transform_point3(QUAD_MODEL[3]);
        self.attack_collider.set_vertices(q0, q1, q2, q3);
    }

    /// `EnMb_PostLimbDraw` for limb `limb` at `mtx` (in the world): the club Moblin's club head
    /// (`effSpawnPos`, 8000 down its left hand) and quad, its feet; the ten body parts for the
    /// ice.
    fn post_limb_draw(&mut self, limb: usize, mtx: &Mat4) {
        const EFF_SPAWN_OFFSET_FROM_LEFT_HAND: Vec3 = Vec3::new(0.0, -8000.0, 0.0);
        if self.club() {
            if limb == ENMB_LIMB_LHAND {
                self.eff_spawn_pos = mtx.transform_point3(EFF_SPAWN_OFFSET_FROM_LEFT_HAND);
                if self.attack > ENMB_ATTACK_NONE {
                    self.club_update_attack_collider(mtx);
                }
            }
            // Actor_SetFeetPos(.., ENMB_LIMB_LFOOT, &feetPos, ENMB_LIMB_RFOOT, &feetPos).
            if limb == ENMB_LIMB_LFOOT {
                self.actor.shape_feet_pos[0] = mtx.transform_point3(Vec3::ZERO);
            } else if limb == ENMB_LIMB_RFOOT {
                self.actor.shape_feet_pos[1] = mtx.transform_point3(Vec3::ZERO);
            }
        }
        if self.ice_effect_timer != 0 {
            let body_part = match limb {
                ENMB_LIMB_HEAD => 0,
                ENMB_LIMB_LHAND => 1,
                ENMB_LIMB_RHAND => 2,
                ENMB_LIMB_LSHOULDER => 3,
                ENMB_LIMB_RSHOULDER => 4,
                ENMB_LIMB_CHEST => 5,
                ENMB_LIMB_LTHIGH => 6,
                ENMB_LIMB_RTHIGH => 7,
                ENMB_LIMB_LFOOT => 8,
                ENMB_LIMB_RFOOT => 9,
                _ => return,
            };
            let p = mtx.transform_point3(Vec3::ZERO);
            self.body_parts_pos[body_part] = [p.x as i32 as i16, p.y as i32 as i16, p.z as i32 as i16];
        }
    }
}

// shared: func_800358DC (z_actor.c)
/// `func_800358DC`: an `En_Part` of kind `params` as `actor`'s child at `spawn_pos` turned
/// `spawn_rot` (x, y; its z the object slot), `actor`'s scale, at `arg3[0]` forward, its list
/// `NULL`, action 2 (a shock wave), `timer` frames, `rotZ` `arg3[1]`, `rotZSpeed` `arg3[2]`.
fn func_800358dc(actor: &mut Actor, play: &mut PlayState, spawn_pos: Vec3, spawn_rot: [i16; 2], arg3: [f32; 3], timer: i16, params: i16) {
    let scale = actor.scale;
    match play.actor_spawn_as_child(actor, ACTOR_EN_PART, spawn_pos, [spawn_rot[0], spawn_rot[1], 0], params) {
        Ok(h) => {
            if let Some(p) = play.actors.downcast_mut::<EnPart>(h) {
                p.actor.scale = scale;
                p.actor.speed_xz = arg3[0];
                p.display_list = None;
                p.action = 2;
                p.timer = timer;
                p.rot_z = arg3[1];
                p.rot_z_speed = arg3[2];
            }
        }
        Err(e) => log::debug!("En_Mb: func_800358DC: {e:?}"),
    }
}

impl ActorImpl for EnMb {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnMb_Update`.
    fn update(&mut self, play: &mut PlayState) {
        self.check_colliding(play);
        if self.actor.col_chk_info.damage_reaction == ENMB_DMG_REACT_FREEZE {
            return;
        }
        self.run_action(play);
        self.actor.move_forward();
        let flags = UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2 | UPDBGCHECKINFO_FLAG_3 | UPDBGCHECKINFO_FLAG_4;
        self.actor.update_bg_check_info(&play.col, 40.0, 40.0, 70.0, flags);
        self.actor.set_focus(self.actor.scale.x * 4500.0);
        self.body_collider.update(&self.actor);
        if self.actor.col_chk_info.health == 0 {
            let d = -4400.0 * self.actor.scale.y;
            let pos = &mut self.body_collider.dim.pos;
            pos[0] = (pos[0] as f32 + sin_s(self.actor.shape_rot.y) * d) as i32 as i16;
            pos[2] = (pos[2] as f32 + cos_s(self.actor.shape_rot.y) * d) as i32 as i16;
        }
        play.collision_check_set_oc(&self.actor, 0, &mut self.body_collider);
        if self.state >= ENMB_STATE_STUNNED && (self.club() || self.state != ENMB_STATE_ATTACK) {
            play.collision_check_set_ac(&self.actor, 0, &mut self.body_collider);
        }
        if self.state >= ENMB_STATE_IDLE {
            // @bug (game): the club Moblin's triangles are never placed (EnMb_Draw sets them for
            // the spear Moblins only): they stay at the init's, by the world's origin.
            play.collision_check_set_ac(&self.actor, 2, &mut self.front_shielding_collider);
        }
        if self.attack > ENMB_ATTACK_NONE {
            play.collision_check_set_at(&self.actor, 1, &mut self.attack_collider);
        }
    }

    /// `EnMb_Draw`'s changes to the actor: `EnMb_PostLimbDraw` over the skeleton's limbs; a spear
    /// Moblin's quad (attacking) and front triangles from the actor's matrix; the ice: the colour
    /// kept on, a piece every 4 frames off the next part down (`EffectSsEnIce_SpawnFlyingVec3s`).
    fn draw_update(&mut self, play: &mut PlayState) {
        let model = actor_draw_matrix(&RenderState::of(&self.actor));
        if let Some(skeleton) = self.skeleton.clone() {
            let joints = self.skel.joint_table.clone();
            let mut posts = Vec::new();
            draw_opa_pose(&skeleton, &joints, |_, _, _| LimbDraw::Default(Mat4::IDENTITY), |limb, m| posts.push((limb, m)));
            for (limb, m) in posts {
                self.post_limb_draw(limb, &(model * m));
            }
        }
        if !self.club() {
            if self.attack > ENMB_ATTACK_NONE {
                self.spear_update_attack_collider(&model);
            }
            const FRONT_SHIELDING_TRI_MODEL0: [Vec3; 3] = [Vec3::new(4000.0, 7000.0, 3500.0), Vec3::new(4000.0, 0.0, 3500.0), Vec3::new(-4000.0, 7000.0, 3500.0)];
            const FRONT_SHIELDING_TRI_MODEL1: [Vec3; 3] = [Vec3::new(-4000.0, 7000.0, 3500.0), Vec3::new(-4000.0, 0.0, 3500.0), Vec3::new(4000.0, 0.0, 3500.0)];
            let t0 = FRONT_SHIELDING_TRI_MODEL0.map(|v| model.transform_point3(v));
            let t1 = FRONT_SHIELDING_TRI_MODEL1.map(|v| model.transform_point3(v));
            self.front_shielding_collider.set_vertices(0, t0[0], t0[1], t0[2]);
            self.front_shielding_collider.set_vertices(1, t1[0], t1[1], t1[2]);
        }
        if self.ice_effect_timer != 0 {
            self.actor.color_filter_timer = self.actor.color_filter_timer.wrapping_add(1);
            if self.ice_effect_timer >= 0 {
                self.ice_effect_timer -= 1;
            }
            if self.ice_effect_timer % 4 == 0 {
                let scale = if self.club() { 4.0 } else { 2.5 };
                let body_part_idx = (self.ice_effect_timer >> 2) as usize;
                let p = self.body_parts_pos.get(body_part_idx).copied().unwrap_or([0; 3]);
                let r = self.actor.shape_rot;
                let me = play.cur_actor.map(|h| oot_game::effect::SsActor { handle: h, world_pos: self.actor.world_pos, shape_rot: [r.x, r.y, r.z] });
                play.effect_ss_en_ice_spawn_flying_vec3f(me, Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32), [150, 150, 150, 250], [235, 245, 255], scale);
            }
        }
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        rs.switches = vec![self.club() as u32];
        rs
    }

    /// `EnMb_Draw`: the skeleton (`SkelAnime_DrawFlexOpa` after `Gfx_SetupDL_25Opa`).
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let (Some(joints), Some(skeleton)) = (&rs.joints, &self.skeleton) else { return };
        let [club] = rs.switches.as_slice() else { return };
        let bones = skeleton.pose_override(&joints.rot, |_, _, _| Mat4::IDENTITY);
        let mesh = MeshKey::named(keys::bake(bake_name(*club != 0)));
        out.opa.push(DrawCmd { mesh, transform: actor_draw_matrix(rs), bones, params: Default::default() });
    }

    /// `EnMb_Destroy`: its colliders (nothing to free here).
    fn destroy(&mut self, _play: &mut PlayState) {}

    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        match id {
            0 => Some(ColliderMut::Cylinder(&mut self.body_collider)),
            1 => Some(ColliderMut::Quad(&mut self.attack_collider)),
            2 => Some(ColliderMut::Tris(&mut self.front_shielding_collider)),
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
