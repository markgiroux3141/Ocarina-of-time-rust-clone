//! `En_Zf` (`ovl_En_Zf/z_en_zf.c`): the Lizalfos and the Dinolfos (GAME-09 milestone 3). A child
//! meets them in Dodongo's Cavern (Master Quest): room 3's two pairs, the lower one (0x0500,
//! 0x0501) on the room's floor (y 100) and the upper one (0x0600, 0x0601) on its upper level (y
//! 531), and room 8's single (0xFFFF). The Water, Fire and Spirit Temples, Ganon's Castle and the
//! Gerudo Training Ground have more (the Dinolfos: an adult's).
//!
//! - **The types** (`EnZfType`, the params' low byte sign-extended): -2 `ENZF_TYPE_DINOLFOS` (12
//!   health, `gDinolfosSkel`, starts idle, dodges and jump-attacks more), -1
//!   `ENZF_TYPE_LIZALFOS_SINGLE` (6 health, waits unseen 300 above its floor until Link is within
//!   160 across, then drops in), 0 and 1 the paired mini-boss (`..._PAIRED_MINIBOSS_A`, `_B`: 6
//!   health each). The high byte is a switch flag (`switchFlag`), the pair's.
//! - **A pair's init** (`EnZf_Init`): one more than 100 above or below Link, or with its switch flag
//!   set, kills itself; else it finds its platform (`EnZf_FindPlatform` against
//!   `sDCPlatformPositions`, room 3's 24 platforms: 8 below, 16 above) and waits 300 up, 10
//!   frames, then drops in (`EnZf_WaitAppear`: `NA_SE_EN_RIZA_CRY`; A starts the mini-boss music,
//!   `func_800F5ACC(NA_BGM_MINI_BOSS)`). A fights (`EnZf_SetupIdle`); B turns and jumps away
//!   (`EnZf_Paired_JumpAwayStartTurnAround`), its sword sheathed, and hops on a platform away from
//!   Link (`EnZf_Paired_IdleHop`), jumping on (`EnZf_Paired_JumpAway`, from platform to platform:
//!   `EnZf_FindPlatformWithoutPlayer`) whenever he comes within 400 (250 above).
//! - **The pair's turns** (`sPairedActiveFleeFlag`, `sPairedInactiveType`): the fighter hit from 4
//!   or more health to under 4 flees once its hurt animation ends (`EnZf_Damaged`); the other is
//!   signalled (`timer2` 90) and after 90 frames turns and draws its sword
//!   (`EnZf_Paired_EngageCombat`). The one waiting can't be hit. The first to die signals the other
//!   (its health back up to 3 if under) and makes it the last (`sPairedInactiveType`
//!   `ENZF_TYPE_LIZALFOS_SINGLE`); the second's death sets the switch flag and puts the music back
//!   (`func_800F5B58`); so does the destroy of the last paired one with no `En_Zf` left.
//! - **The fighting** (all types): idle a few frames, then walking at Link (`EnZf_MainWalk`, on a
//!   pair's platforms jumping across to the one nearest him, `EnZf_FindPlatformCloseToPlayer`), or
//!   side-stepping round him (`EnZf_Sidestep`, `EnZf_CirclePlayer`), turning to him, slashing within
//!   100 (`EnZf_Attack`: its sword's quad hurts 8 at frames 14 to 20, the trail
//!   `EFFECT_BLURE1` drawn behind it; bounced off the shield, it recoils, `EnZf_AttackBounced`),
//!   jumping at him (`EnZf_JumpForwards`) or back (`EnZf_JumpBackwards`), a jump attack
//!   (`EnZf_JumpAttack`). An arrow or a seed flying at it (`Actor_GetProjectileActor`) makes it
//!   dodge (`EnZf_ReactToProjectile`) or jump at Link; the hopping one dodges by its hop
//!   (`EnZf_ReactToProjectileSetDodgeSpeed`). The Dinolfos also reacts to Link's sword and to
//!   explosives within 80 (`EnZf_DinolfosPickAction`). It never walks off a floor's edge (it
//!   doesn't move at all when there's no floor ahead).
//! - **Hit** (`EnZf_CheckCollide`, up to `ENZF_ACTION_STUNNED`: not while hurt, jumping away or
//!   jump-attacking): nuts, the boomerang and the hookshot stun it (blue 80 frames), ice too (and
//!   ice chunks, `EffectSsEnIce`); fire and light magic do nothing (and hold it that frame); other
//!   damage flashes it red 8 frames and hurts it (`EnZf_Damaged`, `NA_SE_EN_RIZA_DAMAGE`). The
//!   Kokiri Sword does 1; 6 hits kill a Lizalfos.
//! - **Death** (`EnZf_Die`): `NA_SE_EN_RIZA_DEAD`, a drop from table 4 (14 for the Dinolfos), a
//!   prop once its animation ends, then fading out 5 a frame.
//!
//! The whole overlay is ported. Its draw (`EnZf_Draw`) is the skeleton (`gLizalfosSkel` or
//! `gDinolfosSkel`), opaque at alpha 255, else translucent (`gActorSetupXluDL` on segment 9, the env
//! alpha), the sword's hand and the sheath swapped for the sheathed lists
//! (`gLizalfosRightHandDL`, `gLizalfosSheathedSwordDL`) while `isSwordSheathed`
//! (`EnZf_OverrideLimbDraw`): eight bakes. `draw_update` is `EnZf_PostLimbDraw`: the sword's
//! quad, the trail's vertices, the feet, the frozen body parts and their ice. The feet shadow
//! (`ActorShadow_DrawFeet`) is a disc under each foot (the usual stand-in); `func_8002EBCC`'s
//! highlight (segment 7, the sword's shine) is its tile at a fixed place, the renderer's texgen. `D_80B4A2F8` (segment 8, a
//! `gSPTexture` with `IREG(0)`, `IREG(1)`) is called by none of the object's lists. Not ported:
//! the bombchu (`En_Bom_Chu`, the Dinolfos's check of one) and the hookshot
//! (`Actor_GetProjectileActor` sees arrows and seeds only).
//!
//! Params: `type` bits 0 to 7, sign-extended (`EnZfType`: -2 the Dinolfos, -1 a single Lizalfos,
//! 0 and 1 the paired mini-boss's A and B), written over the params at init. `switchFlag` bits 8
//! to 15 (a pair's).
//! Flags: a pair reads switch flag `switchFlag` at init (set, it's gone); the second of a pair to
//! die sets it.

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use eng_math::{cos_s, sin_s, smooth_step_to_f, smooth_step_to_s, vec3f_yaw};
use glam::{Mat4, Vec3};
use oot_game::actor::*;
use oot_game::actor_ctx::{
    ACTORCAT_ENEMY, ACTORCAT_EXPLOSIVE, ACTORCAT_PROP, ActorHandle, ActorImpl, ActorProfile, actor_find_nearby, actor_is_locked_on, actor_other_is_locked_on,
    actor_spawn_floor_dust_ring, actor_test_floor_in_direction, audio_play_actor_sfx2, enemy_start_finishing_blow, func_800354b4, func_800355b8,
};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind};
use oot_game::audio::sfx::NA_SE_EN_GOMA_JR_FREEZE;
use oot_game::collision_check::*;
use oot_game::effect::EffectInit;
use oot_game::effect::blure::{EffectBlureInit, EffectBlureInit1};
use oot_game::pack::{BakeBody, BakeSegment, LimbOverride, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::skelanime_std::{ANIMMODE_LOOP, ANIMMODE_LOOP_INTERP, ANIMMODE_ONCE, Anim, LimbDraw, SkelAnimeStd, draw_opa_pose};

use crate::player::{self, Player};

/// `ACTOR_EN_ZF` (`actor_table.h`: 0x0025).
pub const ACTOR_EN_ZF: i16 = 0x0025;
/// `ACTOR_EN_BOM_CHU` (`actor_table.h`: 0x00DA; not ported).
pub const ACTOR_EN_BOM_CHU: i16 = 0x00DA;
/// `OBJECT_ZF` (`object_table.h`: 0x001B).
pub const OBJECT: &str = "object_zf";
const SKEL_LIZALFOS: &str = "gLizalfosSkel";
const SKEL_DINOLFOS: &str = "gDinolfosSkel";

/// `En_Zf_Profile`: `ACTORCAT_ENEMY`, attention-enabled, hostile, always updating.
pub const PROFILE: ActorProfile =
    ActorProfile { id: ACTOR_EN_ZF, name: "En_Zf", category: ACTORCAT_ENEMY, flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE | ACTOR_FLAG_UPDATE_CULLING_DISABLED, object: OBJECT, schema: &SCHEMA };

/// The type: params bits 0 to 7, sign-extended (`PARAMS_GET_U(params, 0, 8)`, `| 0xFF00` with bit
/// 7), written over the params at init.
pub const TYPE: Field = Field::params(
    "type",
    0,
    8,
    Kind::Named(&[(-2, "ENZF_TYPE_DINOLFOS"), (-1, "ENZF_TYPE_LIZALFOS_SINGLE"), (0, "ENZF_TYPE_LIZALFOS_PAIRED_MINIBOSS_A"), (1, "ENZF_TYPE_LIZALFOS_PAIRED_MINIBOSS_B")]),
    "the Dinolfos, a single Lizalfos, or the paired mini-boss's A (fights first) or B",
)
.signed();
/// `switchFlag`: `PARAMS_GET_S(params, 8, 8)`.
pub const SWITCH_FLAG: Field = Field::params("switchFlag", 8, 8, Kind::SwitchFlag, "a pair's: set at the second's death; set at init, the pair is gone");

/// En_Zf's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[TYPE, SWITCH_FLAG],
    flags: &[FlagUse::new(Flag::Switch("switchFlag")).reads().sets().note("a pair: set at init, it's gone; set at the second's death")],
    todo: false,
};

/// `EnZfType` (`z_en_zf.h`).
pub const ENZF_TYPE_DINOLFOS: i16 = -2;
pub const ENZF_TYPE_LIZALFOS_SINGLE: i16 = -1;
pub const ENZF_TYPE_LIZALFOS_PAIRED_MINIBOSS_A: i16 = 0;
pub const ENZF_TYPE_LIZALFOS_PAIRED_MINIBOSS_B: i16 = 1;

/// `ENZF_IS_TYPE_PAIRED`.
pub const fn is_type_paired(ty: i16) -> bool {
    ty >= ENZF_TYPE_LIZALFOS_PAIRED_MINIBOSS_A
}

/// `EnZfAction`.
pub const ENZF_ACTION_WAIT_APPEAR: i32 = 0;
pub const ENZF_ACTION_IDLE: i32 = 3;
pub const ENZF_ACTION_MAIN_WALK: i32 = 5;
pub const ENZF_ACTION_TURN_TOWARDS_PLAYER: i32 = 6;
pub const ENZF_ACTION_SIDESTEP: i32 = 7;
pub const ENZF_ACTION_ATTACK_BOUNCED: i32 = 8;
pub const ENZF_ACTION_ATTACK: i32 = 9;
pub const ENZF_ACTION_JUMP_BACKWARDS: i32 = 11;
pub const ENZF_ACTION_CIRCLE_PLAYER: i32 = 12;
pub const ENZF_ACTION_JUMP_FORWARDS: i32 = 13;
pub const ENZF_ACTION_STUNNED: i32 = 14;
pub const ENZF_ACTION_DIE: i32 = 15;
pub const ENZF_ACTION_DAMAGED: i32 = 16;
pub const ENZF_ACTION_PAIRED_JUMP_AWAY_START_TURN_AROUND: i32 = 18;
pub const ENZF_ACTION_PAIRED_JUMP_AWAY: i32 = 19;
pub const ENZF_ACTION_PAIRED_IDLE_HOP: i32 = 20;
pub const ENZF_ACTION_PAIRED_ENGAGE_COMBAT: i32 = 21;
pub const ENZF_ACTION_JUMP_ATTACK: i32 = 22;

/// `EnZfDamageReaction`.
pub const ENZF_DMG_REACT_NONE: u8 = 0;
pub const ENZF_DMG_REACT_STUN: u8 = 1;
pub const ENZF_DMG_REACT_6: u8 = 6;
pub const ENZF_DMG_REACT_FLEE: u8 = 13;
pub const ENZF_DMG_REACT_ICE: u8 = 15;

// The sounds (`include/tables/sfx/enemybank_table.h`).
/// `NA_SE_EN_RIZA_ONGND` (0x3828).
pub const NA_SE_EN_RIZA_ONGND: u16 = 0x3828;
/// `NA_SE_EN_RIZA_CRY` (0x3829).
pub const NA_SE_EN_RIZA_CRY: u16 = 0x3829;
/// `NA_SE_EN_RIZA_ATTACK` (0x382A).
pub const NA_SE_EN_RIZA_ATTACK: u16 = 0x382A;
/// `NA_SE_EN_RIZA_DAMAGE` (0x382B).
pub const NA_SE_EN_RIZA_DAMAGE: u16 = 0x382B;
/// `NA_SE_EN_RIZA_DEAD` (0x382D).
pub const NA_SE_EN_RIZA_DEAD: u16 = 0x382D;
/// `NA_SE_EN_RIZA_WALK` (0x382E).
pub const NA_SE_EN_RIZA_WALK: u16 = 0x382E;
/// `NA_SE_EN_RIZA_JUMP` (0x382F).
pub const NA_SE_EN_RIZA_JUMP: u16 = 0x382F;
/// `NA_SE_EN_RIZA_DOWN` (0x387A).
pub const NA_SE_EN_RIZA_DOWN: u16 = 0x387A;

/// `NA_BGM_MINI_BOSS` (`sequence_table.h`: `Sequence_56`, 0x38).
pub const NA_BGM_MINI_BOSS: u16 = 0x38;

/// `NAVI_ENEMY_LIZALFOS`, `NAVI_ENEMY_DINOLFOS` (`actor.h`: 0x0F, 0x10).
pub const NAVI_ENEMY_LIZALFOS: u8 = 0x0F;
pub const NAVI_ENEMY_DINOLFOS: u8 = 0x10;

/// `ATTENTION_RANGE_3` (`actor.h`).
const ATTENTION_RANGE_3: u8 = 3;

/// `COLLECTIBLE_DROP_RANDOM_PARAMS(COLLECTIBLE_DROP_TABLE_4 / _14, false)` (`z_en_item00.h`: the
/// table × 16).
const DROP_TABLE_4: i16 = 4 * 16;
const DROP_TABLE_14: i16 = 14 * 16;

/// `ZfLimb` (`object_zf.xml`, 1-based; the Dinolfos's `DinolfosLimb` has the same indices).
pub const ZF_LIMB_HEAD_ROOT: usize = 5;
pub const ZF_LIMB_HEAD: usize = 9;
pub const ZF_LIMB_NECK: usize = 10;
pub const ZF_LIMB_RIGHT_HAND_AND_SWORD_ROOT: usize = 14;
pub const ZF_LIMB_RIGHT_HAND_AND_SWORD: usize = 15;
pub const ZF_LIMB_FOREARM: usize = 16;
pub const ZF_LIMB_LEFT_FOREARM: usize = 23;
pub const ZF_LIMB_ARMOR: usize = 25;
pub const ZF_LIMB_SHEATH: usize = 33;
pub const ZF_LIMB_RIGHT_FOOT_CLAWS: usize = 38;
pub const ZF_LIMB_RIGHT_SHIN: usize = 39;
pub const ZF_LIMB_LEFT_SHIN_ROOT: usize = 43;
pub const ZF_LIMB_LEFT_FOOT_CLAWS: usize = 45;
pub const ZF_LIMB_BODY: usize = 48;
pub const ZF_LIMB_MAX: usize = 49;

/// `sDCPlatformPositions`: the platforms of Dodongo's Cavern's Lizalfos room (room 3): the lower
/// floor's 8, the upper floor's 7, its extras (the centre, the room's sides, the path atop the
/// column).
pub const S_DC_PLATFORM_POSITIONS: [Vec3; 24] = [
    // Lower floor
    Vec3::new(3560.0, 100.0, -1517.0),
    Vec3::new(3170.0, 100.0, -1767.0),
    Vec3::new(3165.0, 100.0, -2217.0),
    Vec3::new(3563.0, 100.0, -2437.0),
    Vec3::new(3946.0, 100.0, -2217.0),
    Vec3::new(3942.0, 100.0, -1765.0),
    Vec3::new(2861.0, 100.0, -2394.0),
    Vec3::new(2776.0, 100.0, -1987.0),
    // Higher floor
    Vec3::new(4527.0, 531.0, -1146.0),
    Vec3::new(4442.0, 531.0, -1405.0),
    Vec3::new(4170.0, 531.0, -1395.0),
    Vec3::new(4030.0, 531.0, -1162.0),
    Vec3::new(4010.0, 531.0, -883.0),
    Vec3::new(4270.0, 531.0, -810.0),
    Vec3::new(4520.0, 531.0, -880.0),
    // Higher floor extras: the centre platform
    Vec3::new(4260.0, 531.0, -1035.0),
    // the room's sides
    Vec3::new(4757.0, 531.0, -1146.0),
    Vec3::new(3850.0, 531.0, -883.0),
    Vec3::new(4380.0, 531.0, -690.0),
    Vec3::new(4197.0, 531.0, -646.0),
    // the "path" to atop the column
    Vec3::new(4070.0, 531.0, -1575.0),
    Vec3::new(3930.0, 531.0, -1705.0),
    Vec3::new(3780.0, 531.0, -1835.0),
    Vec3::new(3560.0, 531.0, -1985.0),
];

/// `sDCPlatformPositions[i]`. @bug (game): -1 (no platform: a search that found none, a home off
/// the platforms) reads the 12 bytes before the table, the section padding between `.text` and
/// `.data`: (0, 0, 0).
pub fn platform_pos(i: i16) -> Vec3 {
    usize::try_from(i).ok().and_then(|i| S_DC_PLATFORM_POSITIONS.get(i).copied()).unwrap_or(Vec3::ZERO)
}

/// The overlay's statics (`PlayState::overlay_static`, by `ACTOR_EN_ZF`), reset as the overlay
/// reloads (its last instance gone: `EnZf::destroy`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Statics {
    /// `sPairedActiveFleeFlag`: counted up when the pair's fighter should retreat.
    pub s_paired_active_flee_flag: i16,
    /// `sPairedInactiveType`: the pair's one not fighting (A or B); `ENZF_TYPE_LIZALFOS_SINGLE`
    /// with no pair, or once one of it is dead.
    pub s_paired_inactive_type: i16,
    /// `D_80B4AB30`: the jump away's frames in the air (counted, never read).
    pub d_80b4ab30: i32,
}

impl Default for Statics {
    /// The `.data`'s: `sPairedInactiveType` starts at `ENZF_TYPE_LIZALFOS_PAIRED_MINIBOSS_B`.
    fn default() -> Self {
        Statics { s_paired_active_flee_flag: 0, s_paired_inactive_type: ENZF_TYPE_LIZALFOS_PAIRED_MINIBOSS_B, d_80b4ab30: 0 }
    }
}

fn statics(play: &mut PlayState) -> &mut Statics {
    play.overlay_static::<Statics>(ACTOR_EN_ZF)
}

/// `sPairedInactiveType`.
fn inactive_type(play: &mut PlayState) -> i16 {
    statics(play).s_paired_inactive_type
}

/// `sBodyCylinderInit`: hit by Player (0xFFCFFFFF), pushed by Player; 20 by 70.
const BODY_CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_HIT0, at_flags: AT_NONE, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_ON | OC1_TYPE_PLAYER, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK1,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0x0000_0000, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x00 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: HIT_BACKLASH_NONE, defense: 0x00 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 20, height: 70, y_shift: 0, pos: [0, 0, 0] },
};

/// `sSwordColliderQuadInit`: the sword hurts 8 (0xFFCFFFFF); hard (never set for AC).
const SWORD_QUAD_INIT: ColliderQuadInit = ColliderQuadInit {
    base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_ON | AT_TYPE_ENEMY, ac_flags: AC_ON | AC_HARD | AC_TYPE_PLAYER, oc_flags1: OC1_NONE, oc_flags2: OC2_NONE, shape: COLSHAPE_QUAD },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0xFFCF_FFFF, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x08 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0x0000_0000, hit_backlash: HIT_BACKLASH_NONE, defense: 0x00 },
        at_elem_flags: ATELEM_ON | ATELEM_SFX_NORMAL | ATELEM_UNK7,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_NONE,
    },
    quad: [Vec3::ZERO; 4],
};

/// The colliders' ids (`collider_mut`).
const COL_BODY: u8 = 0;
const COL_SWORD: u8 = 1;

/// `sDamageTable`.
pub static S_DAMAGE_TABLE: DamageTable = DamageTable {
    table: [
        dmg_entry(0, ENZF_DMG_REACT_STUN), // Deku nut
        dmg_entry(2, ENZF_DMG_REACT_NONE), // Deku stick
        dmg_entry(1, ENZF_DMG_REACT_FLEE), // Slingshot
        dmg_entry(2, ENZF_DMG_REACT_NONE), // Explosive
        dmg_entry(0, ENZF_DMG_REACT_STUN), // Boomerang
        dmg_entry(2, ENZF_DMG_REACT_NONE), // Normal arrow
        dmg_entry(2, ENZF_DMG_REACT_NONE), // Hammer swing
        dmg_entry(0, ENZF_DMG_REACT_STUN), // Hookshot
        dmg_entry(1, ENZF_DMG_REACT_NONE), // Kokiri sword
        dmg_entry(2, ENZF_DMG_REACT_NONE), // Master sword
        dmg_entry(4, ENZF_DMG_REACT_NONE), // Giant's Knife
        dmg_entry(2, ENZF_DMG_REACT_FLEE), // Fire arrow
        dmg_entry(4, ENZF_DMG_REACT_ICE),  // Ice arrow
        dmg_entry(2, ENZF_DMG_REACT_FLEE), // Light arrow
        dmg_entry(2, ENZF_DMG_REACT_FLEE), // Unk arrow 1
        dmg_entry(2, ENZF_DMG_REACT_FLEE), // Unk arrow 2
        dmg_entry(2, ENZF_DMG_REACT_FLEE), // Unk arrow 3
        dmg_entry(0, ENZF_DMG_REACT_6),    // Fire magic
        dmg_entry(3, ENZF_DMG_REACT_ICE),  // Ice magic
        dmg_entry(0, ENZF_DMG_REACT_6),    // Light magic
        dmg_entry(0, ENZF_DMG_REACT_NONE), // Shield
        dmg_entry(0, ENZF_DMG_REACT_NONE), // Mirror Ray
        dmg_entry(1, ENZF_DMG_REACT_NONE), // Kokiri spin
        dmg_entry(4, ENZF_DMG_REACT_NONE), // Giant spin
        dmg_entry(2, ENZF_DMG_REACT_NONE), // Master spin
        dmg_entry(2, ENZF_DMG_REACT_NONE), // Kokiri jump
        dmg_entry(8, ENZF_DMG_REACT_NONE), // Giant jump
        dmg_entry(4, ENZF_DMG_REACT_NONE), // Master jump
        dmg_entry(0, ENZF_DMG_REACT_NONE), // Unknown 1
        dmg_entry(0, ENZF_DMG_REACT_NONE), // Unblockable
        dmg_entry(4, ENZF_DMG_REACT_NONE), // Hammer jump
        dmg_entry(0, ENZF_DMG_REACT_NONE), // Unknown 2
    ],
};

/// `D_80B4A298`: a foot's place on its claws' limb (`Actor_SetFeetPos`, `leftFootPos`,
/// `rightFootPos`).
const D_80B4A298: Vec3 = Vec3::new(300.0, 0.0, 0.0);
/// `D_80B4A2A4`, `D_80B4A2B0`: the trail's two points on the sword's limb.
const D_80B4A2A4: Vec3 = Vec3::new(300.0, -1700.0, 0.0);
const D_80B4A2B0: Vec3 = Vec3::new(-600.0, 300.0, 0.0);
/// `D_80B4A2BC`, `D_80B4A2C8`, `D_80B4A2D4`, `D_80B4A2E0`: the sword quad's corners 1, 0, 3, 2.
const D_80B4A2BC: Vec3 = Vec3::new(0.0, 1500.0, 0.0);
const D_80B4A2C8: Vec3 = Vec3::new(-600.0, -3000.0, 1000.0);
const D_80B4A2D4: Vec3 = Vec3::new(-600.0, -3000.0, -1000.0);
const D_80B4A2E0: Vec3 = Vec3::new(1500.0, -3000.0, 0.0);
/// `D_80B4A2EC`: a frozen body part's place on its limb.
const D_80B4A2EC: Vec3 = Vec3::ZERO;
/// `D_80B4A28C`: unused.
#[allow(dead_code)]
const D_80B4A28C: Vec3 = Vec3::new(1100.0, -700.0, 0.0);

/// The segment `EnZf_Draw` puts the setup list on (`ACTOR_SETUP_OPA_DL` or `gActorSetupXluDL`).
const SEG_SETUP: u8 = 0x09;
/// The draw's `gDPSetEnvColor(0, 0, 0, alpha)` before the skeleton, as a dynamic colour the bake
/// runs first.
const SEG_ALPHA: u8 = 0x0E;
/// `func_8002EBCC(&this->actor, play, 1)`'s segment: the highlight's tile (the sword's shine,
/// `gLizalfosRightHandAndSwordDL`'s call).
const SEG_HILITE: u8 = 0x07;

/// `func_8002EBCC`'s list: `gDPSetHilite1Tile(1, hilite, 16, 16)`, `gsSPEndDisplayList()`, with
/// the highlight at (0, 0): its move with the view and the light (`func_8002EABC`) isn't
/// modelled (the renderer's texgen reads the view).
fn hilite_dl() -> Vec<(u32, u32)> {
    let mut d = oot_game::gbi::Dl::default();
    d.set_tile_size(1, 0, 0, (16 - 1) * 4, (16 - 1) * 4);
    d.end();
    d.0
}

// shared: gActorSetupXluDL (z_actor.c; demo_sa.rs has the same list)
/// `gActorSetupXluDL`: `gsDPSetRenderMode(G_RM_FOG_SHADE_A, G_RM_AA_ZB_XLU_SURF2 | Z_UPD)`,
/// `gsDPSetAlphaCompare(G_AC_THRESHOLD)`, `gsSPEndDisplayList()`. `ACTOR_SETUP_OPA_DL` is its
/// third command alone (`&gActorSetupXluDL[2]`: the end).
fn actor_setup_xlu_dl() -> Vec<(u32, u32)> {
    let mut d = oot_game::gbi::Dl::default();
    d.0.push((0xE200_001C, 0xC810_49F8));
    d.othermode_l(0, 2, 1);
    d.end();
    d.0
}

/// The bake of a skeleton (`dinolfos`), with the sheathed sword's lists (`sheathed`), opaque or
/// translucent (`xlu`).
pub fn bake_name(dinolfos: bool, sheathed: bool, xlu: bool) -> String {
    format!("En_Zf/{}{}_{}", if dinolfos { "dinolfos" } else { "lizalfos" }, if sheathed { "_sheathed" } else { "" }, if xlu { "xlu" } else { "opa" })
}

/// `EnZf_Draw`'s meshes: each skeleton after `Gfx_SetupDL_25Opa` / `25Xlu`, its sword drawn or
/// sheathed (`EnZf_OverrideLimbDraw`), opaque or translucent.
pub fn bakes() -> Vec<MeshBake> {
    let mut v = Vec::new();
    for dinolfos in [false, true] {
        for sheathed in [false, true] {
            for xlu in [false, true] {
                let limbs = if sheathed {
                    vec![
                        LimbOverride { limb: (ZF_LIMB_RIGHT_HAND_AND_SWORD - 1) as u8, file: OBJECT.into(), symbol: "gLizalfosRightHandDL".into() },
                        LimbOverride { limb: (ZF_LIMB_SHEATH - 1) as u8, file: OBJECT.into(), symbol: "gLizalfosSheathedSwordDL".into() },
                    ]
                } else {
                    Vec::new()
                };
                v.push(MeshBake {
                    name: bake_name(dinolfos, sheathed, xlu),
                    object: OBJECT.into(),
                    segments: vec![
                        (SEG_SETUP, BakeSegment::Commands(if xlu { actor_setup_xlu_dl() } else { vec![(0xDF00_0000, 0)] })),
                        (SEG_ALPHA, BakeSegment::DynamicColor { env: true, prim: false }),
                        (SEG_HILITE, BakeSegment::Commands(hilite_dl())),
                    ],
                    prelude: vec![SEG_ALPHA],
                    body: BakeBody::Skeleton { file: OBJECT.into(), symbol: if dinolfos { SKEL_DINOLFOS } else { SKEL_LIZALFOS }.into(), limbs },
                });
            }
        }
    }
    v
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Func {
    WaitAppear,
    Idle,
    MainWalk,
    JumpForwards,
    TurnTowardsPlayer,
    Sidestep,
    Attack,
    AttackBounced,
    JumpBackwards,
    Stunned,
    PairedJumpAwayStartTurnAround,
    PairedIdleHop,
    PairedJumpAway,
    PairedEngageCombat,
    Damaged,
    JumpAttack,
    CirclePlayer,
    Die,
}

impl Func {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Func::WaitAppear => "EnZf_WaitAppear",
            Func::Idle => "EnZf_Idle",
            Func::MainWalk => "EnZf_MainWalk",
            Func::JumpForwards => "EnZf_JumpForwards",
            Func::TurnTowardsPlayer => "EnZf_TurnTowardsPlayer",
            Func::Sidestep => "EnZf_Sidestep",
            Func::Attack => "EnZf_Attack",
            Func::AttackBounced => "EnZf_AttackBounced",
            Func::JumpBackwards => "EnZf_JumpBackwards",
            Func::Stunned => "EnZf_Stunned",
            Func::PairedJumpAwayStartTurnAround => "EnZf_Paired_JumpAwayStartTurnAround",
            Func::PairedIdleHop => "EnZf_Paired_IdleHop",
            Func::PairedJumpAway => "EnZf_Paired_JumpAway",
            Func::PairedEngageCombat => "EnZf_Paired_EngageCombat",
            Func::Damaged => "EnZf_Damaged",
            Func::JumpAttack => "EnZf_JumpAttack",
            Func::CirclePlayer => "EnZf_CirclePlayer",
            Func::Die => "EnZf_Die",
        }
    }
}

struct Anims {
    walk: Anim,
    land_on_feet: Anim,
    jump: Anim,
    slash: Anim,
    idle: Anim,
    /// `D_80B4A280`: `gZfRunAwayJumpAnim`, `gZfRunAwayAirborneAnim`, `gZfRunAwayLandAnim`.
    run_away: [Anim; 3],
    run_away_turn_around: Anim,
    turn_around_unsheath_sword: Anim,
    die: Anim,
    damaged: Anim,
    walk_sideways: Anim,
}

impl Anims {
    fn load(play: &PlayState) -> Option<Anims> {
        let a = play.assets.clone()?;
        let get = |s: &str| a.animation(OBJECT, s).map_err(|e| log::error!("En_Zf: {e:#}")).ok();
        Some(Anims {
            walk: get("gZfWalkAnim")?,
            land_on_feet: get("gZfLandOnFeetAnim")?,
            jump: get("gZfJumpAnim")?,
            slash: get("gZfSlashAnim")?,
            idle: get("gZfIdleAnim")?,
            run_away: [get("gZfRunAwayJumpAnim")?, get("gZfRunAwayAirborneAnim")?, get("gZfRunAwayLandAnim")?],
            run_away_turn_around: get("gZfRunAwayTurnAroundAnim")?,
            turn_around_unsheath_sword: get("gZfTurnAroundUnsheathSword")?,
            die: get("gZfDieAnim")?,
            damaged: get("gZfDamagedAnim")?,
            walk_sideways: get("gZfWalkSidewaysAnim")?,
        })
    }
}

/// What it reads of Player (`GET_PLAYER(play)`).
#[derive(Default, Clone, Copy)]
struct PlayerView {
    pos: Vec3,
    rot_y: i16,
    state1: u32,
    melee_weapon_state: i8,
    focus_actor: Option<ActorHandle>,
}

fn player_view(play: &PlayState) -> PlayerView {
    play.player
        .and_then(|h| play.actors.downcast::<Player>(h))
        .map(|p| PlayerView { pos: p.actor.world_pos, rot_y: p.actor.shape_rot.y, state1: p.state1, melee_weapon_state: p.melee_weapon_state, focus_actor: p.focus_actor })
        .unwrap_or_default()
}

/// `ABS` of an s16 in an expression (an `int`: -0x8000 is 0x8000).
fn abs_i(v: i16) -> i32 {
    (v as i32).abs()
}

/// `ABS` of an s16 stored back in an s16 (-0x8000 stays -0x8000).
fn abs16(v: i16) -> i16 {
    (v as i32).abs() as i16
}

/// `Math_Vec3f_DistXZ` (`Actor_WorldDistXZToPoint`).
fn dist_xz(a: Vec3, b: Vec3) -> f32 {
    let (dx, dz) = (b.x - a.x, b.z - a.z);
    (dx * dx + dz * dz).sqrt()
}

/// `EnZf_FindPlatform`: the platform `pos` is on (within 150 up or down and 210 across, 110 at 420
/// up and over): `preferred` if it's on that one, else the last in the table it's on; -1 for none.
pub fn find_platform(pos: Vec3, preferred: i16) -> i16 {
    let range = if pos.y >= 420.0 { 110.0 } else { 210.0 };
    let on = |p: Vec3| (p.y - 150.0) <= pos.y && pos.y <= (p.y + 150.0) && (p.x - range) <= pos.x && pos.x <= (p.x + range) && (p.z - range) <= pos.z && pos.z <= (p.z + range);
    if preferred != -1 && on(platform_pos(preferred)) {
        return preferred;
    }
    let mut i = S_DC_PLATFORM_POSITIONS.len() as i16 - 1;
    while i >= 0 {
        if on(S_DC_PLATFORM_POSITIONS[i as usize]) {
            break;
        }
        i -= 1;
    }
    i
}

/// `EnZf_FindPlatformWithoutPlayer`: the nearest platform within 585 of `this_pos` (380 on the
/// upper floor's first 15, 400 from its extras) that isn't `cur_platform` nor Link's (nor, Link on
/// none, within 400 of him: 50 above), the second nearest if the nearest is towards Link (within
/// 0x36B0 of his direction); `fallback_platform` with none.
pub fn find_platform_without_player(this_pos: Vec3, cur_platform: i16, fallback_platform: i16, player_pos: Vec3) -> i16 {
    let mut max_platform_dist = 585.0;
    let mut min_platform_dist_from_player = 400.0;
    let mut closest_platform_dist = 99998.0;
    let mut i: i16 = 5;
    let mut i_min: i16 = 0;
    let mut second_closest_platform_dist = 99999.0;
    let mut second_closest_platform: i16 = -1;
    let mut closest_platform: i16 = -1;
    let player_platform = find_platform(player_pos, cur_platform);
    if this_pos.y > 420.0 {
        min_platform_dist_from_player = 50.0;
        i_min = 8;
        if cur_platform >= 15 {
            max_platform_dist = 400.0;
            i = 23;
        } else {
            max_platform_dist = 380.0;
            i = 14;
        }
    }
    while i >= i_min {
        let p = S_DC_PLATFORM_POSITIONS[i as usize];
        let skip = i == cur_platform || i == player_platform || (player_platform == -1 && player_pos.distance(p) < min_platform_dist_from_player);
        if !skip {
            let platform_dist = this_pos.distance(p);
            if platform_dist <= max_platform_dist {
                if platform_dist < closest_platform_dist {
                    second_closest_platform_dist = closest_platform_dist;
                    second_closest_platform = closest_platform;
                    closest_platform_dist = platform_dist;
                    closest_platform = i;
                } else if platform_dist < second_closest_platform_dist {
                    second_closest_platform_dist = platform_dist;
                    second_closest_platform = i;
                }
            }
        }
        i -= 1;
    }
    let closest_platform_pos = platform_pos(closest_platform);
    // (Two Math_Vec3f_DistXYZ calls whose results go unused; the second reads
    // sDCPlatformPositions[-1] when there's no second: the section padding, 0.)
    // @bug (game): the check should be >= 0: platform 0 is never taken as the second.
    if second_closest_platform > 0 {
        let yaw_towards_closest_platform = vec3f_yaw(this_pos, closest_platform_pos);
        let yaw_diff = yaw_towards_closest_platform.wrapping_sub(vec3f_yaw(this_pos, player_pos));
        if abs_i(yaw_diff) < 0x36B0 {
            closest_platform = second_closest_platform;
        }
    }
    if closest_platform < 0 {
        closest_platform = fallback_platform;
    }
    closest_platform
}

/// `EnZf_FindPlatformCloseToPlayer`: of the platforms within 500 of `this_pos` (290 above 200 up:
/// the upper floor's), searched from the last down, Link's own (the search stops there) or the
/// one nearest him; the second nearest if the nearest's second is Link's.
pub fn find_platform_close_to_player(this_pos: Vec3, cur_platform: i16, fallback_platform: i16, player_pos: Vec3) -> i16 {
    let mut i: i16 = 7;
    let mut i_min: i16 = 0;
    let mut player_platform = find_platform(player_pos, -1);
    let mut max_platform_dist = 500.0;
    let mut closest_platform_to_player_dist = 99998.0;
    let mut second_closest_platform_to_player_dist = 99999.0;
    let mut closest_platform_to_player = cur_platform;
    let mut second_closest_platform_to_player = fallback_platform;
    if this_pos.y > 200.0 {
        max_platform_dist = 290.0;
        i = 23;
        i_min = 8;
    }
    while i >= i_min {
        let p = S_DC_PLATFORM_POSITIONS[i as usize];
        if this_pos.distance(p) <= max_platform_dist {
            if i != player_platform {
                let platform_dist_to_player = player_pos.distance(p);
                if platform_dist_to_player < closest_platform_to_player_dist {
                    second_closest_platform_to_player_dist = closest_platform_to_player_dist;
                    second_closest_platform_to_player = closest_platform_to_player;
                    closest_platform_to_player_dist = platform_dist_to_player;
                    closest_platform_to_player = i;
                } else if platform_dist_to_player < second_closest_platform_to_player_dist {
                    second_closest_platform_to_player_dist = platform_dist_to_player;
                    second_closest_platform_to_player = i;
                }
            } else {
                closest_platform_to_player = player_platform;
                // (The decomp's note: this break makes the result depend on the table's order.)
                break;
            }
        }
        i -= 1;
    }
    if second_closest_platform_to_player != player_platform {
        player_platform = closest_platform_to_player;
    } else {
        player_platform = second_closest_platform_to_player;
    }
    player_platform
}

pub struct EnZf {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    skeleton: Option<Arc<Skeleton>>,
    anims: Option<Anims>,
    /// `action` (`EnZfAction`).
    pub action: i32,
    /// `unk_3E0`: set to 0 at init, never read.
    pub unk_3e0: i32,
    pub action_state: i32,
    pub func: Func,
    pub head_rot: i16,
    pub head_rot_step: i16,
    pub timer: i32,
    /// `timer2`: the Dinolfos's head shake after a slash; a pair's signal to engage (set to 90 by
    /// the other one).
    pub timer2: i16,
    pub frozen_timer: i16,
    pub no_floor_ahead: bool,
    pub is_sword_sheathed: bool,
    pub switch_flag: i16,
    pub cur_platform: i16,
    pub home_platform: i16,
    pub target_platform: i16,
    pub alpha: u8,
    pub dodge_speed: f32,
    pub dodge_velocity_y: f32,
    pub last_damage_reaction: u8,
    /// `swordBlureEffectIndex`: the sword's trail (`TOTAL_EFFECT_COUNT` with no slot).
    pub sword_blure_effect_index: usize,
    pub body_collider: ColliderCylinder,
    pub sword_collider: ColliderQuad,
    pub right_foot_pos: Vec3,
    pub left_foot_pos: Vec3,
    pub body_parts_pos: [Vec3; 9],
    /// `shape.shadowAlpha`.
    pub shadow_alpha: u8,
}

impl EnZf {
    fn ty(&self) -> i16 {
        TYPE.get(self.actor.params)
    }

    fn paired(&self) -> bool {
        is_type_paired(self.ty())
    }

    fn dinolfos(&self) -> bool {
        self.ty() == ENZF_TYPE_DINOLFOS
    }

    fn anim(&self, f: impl Fn(&Anims) -> &Anim) -> Option<Anim> {
        self.anims.as_ref().map(|a| f(a).clone())
    }

    /// `Animation_GetLastFrame`.
    fn last_frame(&self, f: impl Fn(&Anims) -> &Anim) -> f32 {
        self.anims.as_ref().map(|a| f(a).last_frame()).unwrap_or(0.0)
    }

    /// `Animation_Change(anim, speed, start, end, mode, morph)`; `end` `None` the last frame.
    fn change(&mut self, f: impl Fn(&Anims) -> &Anim, speed: f32, start: f32, end: Option<f32>, mode: u8, morph: f32) {
        if let Some(a) = self.anim(f) {
            let end = end.unwrap_or_else(|| a.last_frame());
            self.skel.change(a, speed, start, end, mode, morph);
        }
    }

    /// `Animation_MorphToLoop`.
    fn morph_to_loop(&mut self, f: impl Fn(&Anims) -> &Anim, morph: f32) {
        self.change(f, 1.0, 0.0, None, ANIMMODE_LOOP, morph);
    }

    /// `Animation_PlayLoop`.
    fn play_loop(&mut self, f: impl Fn(&Anims) -> &Anim) {
        self.change(f, 1.0, 0.0, None, ANIMMODE_LOOP, 0.0);
    }

    /// `Animation_PlayOnceSetSpeed`.
    fn play_once_set_speed(&mut self, f: impl Fn(&Anims) -> &Anim, speed: f32) {
        self.change(f, speed, 0.0, None, ANIMMODE_ONCE, 0.0);
    }

    /// `D_80B4A280[i]`.
    fn run_away_anim(i: i32) -> impl Fn(&Anims) -> &Anim {
        move |a: &Anims| &a.run_away[i.clamp(0, 2) as usize]
    }

    /// `EnZf_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // sInitChain: lockOnArrowOffset 2000, scale 15 / 1000, gravity -3.5.
        actor.target_arrow_offset = 2000.0;
        actor.scale = Vec3::splat(0.015);
        actor.gravity = -3.5;
        actor.target_mode = ATTENTION_RANGE_3;
        let switch_flag = SWITCH_FLAG.get(actor.params);
        // PARAMS_GET_U(params, 0, 8), sign-extended from bit 7.
        actor.params = TYPE.get(actor.params);
        // ActorShape_Init(&shape, 0, ActorShadow_DrawFeet, 90).
        actor.shape_y_offset = 0.0;
        actor.col_chk_info.mass = MASS_HEAVY;
        actor.col_chk_info.damage_table = Some(&S_DAMAGE_TABLE);
        // EFFECT_BLURE1: white, p1 alpha 255 and p2 64 to 0, elemDuration 8, unkFlag 0, calcMode 2.
        let sword_blure_effect_index = play.effect_add(EffectInit::Blure(EffectBlureInit::One(EffectBlureInit1 {
            p1_start_color: [255, 255, 255, 255],
            p2_start_color: [255, 255, 255, 64],
            p1_end_color: [255, 255, 255, 0],
            p2_end_color: [255, 255, 255, 0],
            elem_duration: 8,
            unk_flag: 0,
            calc_mode: 2,
        })));
        let flags = UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2 | UPDBGCHECKINFO_FLAG_3 | UPDBGCHECKINFO_FLAG_4;
        actor.update_bg_check_info(&play.col, 75.0, 45.0, 45.0, flags);
        actor.col_chk_info.cyl_radius = 40;
        actor.col_chk_info.cyl_height = 100;
        let ty = TYPE.get(actor.params);
        let skel_name = if ty == ENZF_TYPE_DINOLFOS {
            actor.col_chk_info.health = 12;
            actor.navi_enemy_id = NAVI_ENEMY_DINOLFOS;
            SKEL_DINOLFOS
        } else {
            actor.col_chk_info.health = 6;
            actor.navi_enemy_id = NAVI_ENEMY_LIZALFOS;
            SKEL_LIZALFOS
        };
        let skeleton = play.assets.clone().and_then(|a| a.skeleton(OBJECT, skel_name).map_err(|e| log::error!("En_Zf: {e:#}")).ok());
        let anims = Anims::load(play);
        // SkelAnime_Init(.., gZfIdleAnim, .., ZF_LIMB_MAX).
        let skel = SkelAnimeStd::init_flex(ZF_LIMB_MAX - 1, anims.as_ref().map(|a| a.idle.clone()));
        let mut this = EnZf {
            actor,
            skel,
            skeleton,
            anims,
            action: ENZF_ACTION_WAIT_APPEAR,
            unk_3e0: 0,
            action_state: 0,
            func: Func::WaitAppear,
            head_rot: 0,
            head_rot_step: 0,
            timer: 0,
            timer2: 0,
            frozen_timer: 0,
            no_floor_ahead: false,
            is_sword_sheathed: false,
            switch_flag,
            cur_platform: 0,
            home_platform: 0,
            target_platform: 0,
            alpha: 255,
            dodge_speed: 0.0,
            dodge_velocity_y: 0.0,
            last_damage_reaction: 0,
            sword_blure_effect_index,
            body_collider: ColliderCylinder::new(&BODY_CYLINDER_INIT),
            sword_collider: ColliderQuad::new(&SWORD_QUAD_INIT),
            right_foot_pos: Vec3::ZERO,
            left_foot_pos: Vec3::ZERO,
            body_parts_pos: [Vec3::ZERO; 9],
            shadow_alpha: 255,
        };
        if !is_type_paired(ty) {
            this.cur_platform = -1;
            this.home_platform = -1;
            statics(play).s_paired_inactive_type = ENZF_TYPE_LIZALFOS_SINGLE;
            this.action_state = 1;
            if ty == ENZF_TYPE_LIZALFOS_SINGLE {
                this.setup_wait_appear();
            } else {
                // ENZF_TYPE_DINOLFOS
                this.setup_idle(play);
            }
        } else {
            let y_diff = player_view(play).pos.y - this.actor.world_pos.y;
            if y_diff.abs() <= 100.0 && !play.flags.get_switch(this.switch_flag as i32) {
                this.cur_platform = find_platform(this.actor.world_pos, 0);
                this.home_platform = this.cur_platform;
                this.setup_wait_appear();
                statics(play).s_paired_inactive_type = ENZF_TYPE_LIZALFOS_PAIRED_MINIBOSS_B;
            } else {
                this.actor.kill();
            }
        }
        Box::new(this)
    }

    /// `EnZf_TestNoFloorAhead`: no ground `distance` ahead along `world.rot.y` (0: 45 for a pair,
    /// 30 else, the way its speed goes): `Actor_UpdateBgCheckInfo` there (the floor, water), the
    /// position and the flags put back.
    fn test_no_floor_ahead(&mut self, play: &PlayState, mut distance: f32) -> bool {
        if distance == 0.0 {
            distance = if self.actor.speed_xz >= 0.0 { 1.0 } else { -1.0 };
            if self.paired() {
                distance *= 45.0;
            } else {
                distance *= 30.0;
            }
        }
        self.no_floor_at(play, self.actor.world_rot.y, distance)
    }

    /// The test of both `EnZf_TestNoFloorAhead` and `_Shape`: towards `yaw`.
    fn no_floor_at(&mut self, play: &PlayState, yaw: i16, distance: f32) -> bool {
        let prev_pos = self.actor.world_pos;
        let prev_bg_check_flags = self.actor.bg_check_flags;
        self.actor.world_pos.x += sin_s(yaw) * distance;
        self.actor.world_pos.z += cos_s(yaw) * distance;
        self.actor.update_bg_check_info(&play.col, 0.0, 0.0, 0.0, UPDBGCHECKINFO_FLAG_2 | UPDBGCHECKINFO_FLAG_3 | UPDBGCHECKINFO_FLAG_4);
        self.actor.world_pos = prev_pos;
        let ret = self.actor.bg_check_flags & BGCHECKFLAG_GROUND == 0;
        self.actor.bg_check_flags = prev_bg_check_flags;
        ret
    }

    /// `EnZf_TestNoFloorAheadShape`: no ground at its speed ahead (`world.rot.y`), or `distance`
    /// along its facing (`shape.rot.y`).
    fn test_no_floor_ahead_shape(&mut self, play: &PlayState, distance: f32) -> bool {
        if self.actor.speed_xz != 0.0 && self.test_no_floor_ahead(play, self.actor.speed_xz) {
            return true;
        }
        self.no_floor_at(play, self.actor.shape_rot.y, distance)
    }

    /// `Actor_IsFacingPlayer`.
    fn is_facing_player(&self, max_angle: i16) -> bool {
        abs_i(self.actor.yaw_towards_player.wrapping_sub(self.actor.shape_rot.y)) < max_angle as i32
    }

    /// `Actor_WorldYawTowardPoint`.
    fn yaw_toward_point(&self, p: Vec3) -> i16 {
        vec3f_yaw(self.actor.world_pos, p)
    }

    /// `this->actor.prev`, `this->actor.next`: the actors on either side of it in its category's
    /// list (newest first: `prev` spawned after it).
    fn prev_next(&self, play: &PlayState) -> (Option<ActorHandle>, Option<ActorHandle>) {
        let Some(me) = play.cur_actor else { return (None, None) };
        let list = play.actors.category(self.actor.category);
        let Some(i) = list.iter().position(|&h| h == me) else { return (None, None) };
        (i.checked_sub(1).map(|j| list[j]), list.get(i + 1).copied())
    }

    /// `((EnZf*)other)->timer2 = 90`: the signal to the other of the pair to engage. @bug (game):
    /// the C writes it into whatever actor is next to it in the list; here only an `En_Zf` takes it.
    fn signal_engage(play: &mut PlayState, other: ActorHandle) {
        match play.actors.downcast_mut::<EnZf>(other) {
            Some(z) => z.timer2 = 90,
            None => log::debug!("En_Zf: the actor next to it isn't an En_Zf: the C writes timer2 (0x3F4) into it"),
        }
    }

    /// `EnZf_CanAttack`: a pair attacks unless Link is in `PLAYER_STATE1_13`/`_14`; the others
    /// unless Link's locked on to another enemy (the Dinolfos: not to an `En_Zf`, or one flashing).
    fn can_attack(&self, play: &PlayState) -> bool {
        let player = player_view(play);
        if self.paired() {
            return player.state1 & (player::STATE1_13 | player::STATE1_14) == 0;
        }
        if !actor_other_is_locked_on(play, &self.actor) {
            return true;
        }
        if self.dinolfos() {
            let Some(focus) = player.focus_actor.and_then(|h| play.actors.actor(h)) else { return false };
            if focus.category != ACTORCAT_ENEMY {
                return true;
            }
            if focus.id != ACTOR_EN_ZF {
                return false;
            }
            if focus.color_filter_timer != 0 {
                return true;
            }
        }
        false
    }

    /// `EnZf_PickAction`.
    fn pick_action(&mut self, play: &mut PlayState) {
        let abs_rel_yaw_towards_player = abs16(self.actor.yaw_towards_player.wrapping_sub(self.actor.shape_rot.y));
        if abs_rel_yaw_towards_player >= 0x1B58 {
            self.setup_circle_player_if_floor(play);
        } else if self.actor.xz_dist_to_player <= 100.0 && play.gameplay_frames % 8 != 0 && self.can_attack(play) {
            self.setup_attack(play);
        } else {
            self.setup_idle(play);
        }
    }

    /// `EnZf_DinolfosPickAction`: Link's sword swung at it within 100: a jump attack (near a wall
    /// within 80, or every other frame within 90) or a jump back; an explosive within 80: round
    /// Link (a bombchu, or a wall behind: a jump attack at a bombchu coming at it) or a jump back.
    /// Whether it picked.
    fn dinolfos_pick_action(&mut self, play: &mut PlayState) -> bool {
        let abs_rel_wall_yaw = abs16(self.actor.wall_yaw.wrapping_sub(self.actor.shape_rot.y));
        if func_800354b4(play, &self.actor, 100.0, 0x5DC0, 0x2AA8, self.actor.shape_rot.y) {
            self.actor.shape_rot.y = self.actor.yaw_towards_player;
            self.actor.world_rot.y = self.actor.yaw_towards_player;
            if self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 && abs_i(abs_rel_wall_yaw) < 0x2EE0 && self.actor.xz_dist_to_player < 80.0 {
                self.setup_jump_attack(play);
            } else if self.actor.xz_dist_to_player < 90.0 && play.gameplay_frames % 2 != 0 {
                self.setup_jump_attack(play);
            } else {
                self.setup_jump_backwards(play);
            }
            return true;
        }
        if let Some(explosive) = actor_find_nearby(play, &self.actor, play.cur_actor, -1, ACTORCAT_EXPLOSIVE, 80.0) {
            let (e_id, e_pos, e_rot_y) = play.actors.actor(explosive).map(|e| (e.id, e.world_pos, e.world_rot.y)).unwrap_or_default();
            self.actor.shape_rot.y = self.actor.yaw_towards_player;
            self.actor.world_rot.y = self.actor.yaw_towards_player;
            // (absRelWallYaw < 0x2EE0 compares the s16: -0x8000 passes.)
            if (self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 && abs_rel_wall_yaw < 0x2EE0) || e_id == ACTOR_EN_BOM_CHU {
                if e_id == ACTOR_EN_BOM_CHU && self.actor.world_pos.distance(e_pos) < 80.0 && self.actor.shape_rot.y.wrapping_sub(e_rot_y).wrapping_add(-0x8000i32 as i16) < 0x3E80 {
                    self.setup_jump_attack(play);
                } else {
                    self.setup_circle_player(play, 4.0);
                }
            } else {
                self.setup_jump_backwards(play);
            }
            return true;
        }
        false
    }

    /// `EnZf_SetupWaitAppear`: 300 above its floor, `gZfJumpAnim` held at frame 9, unseen and not
    /// targetable, 10 frames (a single: until Link's within 160).
    fn setup_wait_appear(&mut self) {
        let last = self.last_frame(|a| &a.jump);
        self.change(|a| &a.jump, 0.0, 9.0, Some(last), ANIMMODE_LOOP, 0.0);
        self.actor.world_pos.y = self.actor.floor_height + 300.0;
        self.alpha = 0;
        self.shadow_alpha = 0;
        self.timer = 10;
        self.action_state = 1;
        self.action = ENZF_ACTION_WAIT_APPEAR;
        self.actor.bg_check_flags &= !BGCHECKFLAG_GROUND_TOUCH;
        self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        self.actor.shape_rot.y = self.actor.yaw_towards_player;
        self.actor.world_rot.y = self.actor.yaw_towards_player;
        self.func = Func::WaitAppear;
    }

    /// `EnZf_WaitAppear`.
    fn wait_appear(&mut self, play: &mut PlayState) {
        let ty = self.ty();
        if self.timer == 1 {
            audio_play_actor_sfx2(play, NA_SE_EN_RIZA_CRY);
            self.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED;
            if ty == ENZF_TYPE_LIZALFOS_PAIRED_MINIBOSS_A {
                play.audio.func_800f5acc(NA_BGM_MINI_BOSS);
            }
        }
        if self.timer != 0 {
            if ty != ENZF_TYPE_LIZALFOS_SINGLE {
                self.timer -= 1;
            } else if self.actor.xz_dist_to_player <= 160.0 {
                self.timer = 0;
                self.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED;
                audio_play_actor_sfx2(play, NA_SE_EN_RIZA_CRY);
            }
            self.actor.world_pos.y = self.actor.floor_height + 300.0;
        } else if self.alpha < 255 {
            self.alpha = self.alpha.wrapping_add(51);
        }
        if self.actor.bg_check_flags & (BGCHECKFLAG_GROUND | BGCHECKFLAG_GROUND_TOUCH) != 0 && self.action_state != 0 {
            audio_play_actor_sfx2(play, NA_SE_EN_RIZA_ONGND);
            self.change(|a| &a.land_on_feet, 1.0, 0.0, Some(17.0), ANIMMODE_ONCE, 0.0);
            self.action_state = 0;
            self.actor.bg_check_flags &= !BGCHECKFLAG_GROUND_TOUCH;
            self.actor.world_pos.y = self.actor.floor_height;
            self.actor.velocity.y = 0.0;
            self.feet_dust(play);
        }
        if self.skel.update() {
            self.alpha = 255;
            if ty >= ENZF_TYPE_LIZALFOS_PAIRED_MINIBOSS_B {
                self.paired_setup_jump_away_start_turn_around(play);
            } else {
                self.setup_idle(play);
            }
        }
        self.shadow_alpha = self.alpha;
    }

    /// The landing's dust at both feet (`Actor_SpawnFloorDustRing(.., 3, 2, 2, 0, 0, false)`).
    fn feet_dust(&self, play: &mut PlayState) {
        actor_spawn_floor_dust_ring(play, &self.actor, self.left_foot_pos, 3.0, 2, 2.0, 0, 0, false);
        actor_spawn_floor_dust_ring(play, &self.actor, self.right_foot_pos, 3.0, 2, 2.0, 0, 0, false);
    }

    /// `EnZf_SetupIdle`: 5 to 14 frames.
    fn setup_idle(&mut self, play: &mut PlayState) {
        self.change(|a| &a.idle, 1.0, 0.0, None, ANIMMODE_LOOP_INTERP, -4.0);
        self.action = ENZF_ACTION_IDLE;
        self.timer = ((play.rand.zero_one() * 10.0) + 5.0) as i32;
        self.actor.speed_xz = 0.0;
        self.actor.world_rot.y = self.actor.shape_rot.y;
        self.func = Func::Idle;
    }

    /// `EnZf_Idle`.
    fn idle(&mut self, play: &mut PlayState) {
        let player = player_view(play);
        let abs_yaw_towards_player_rel_to_head = abs16(self.actor.yaw_towards_player.wrapping_sub(self.head_rot).wrapping_sub(self.actor.shape_rot.y));
        self.skel.update();
        if self.react_to_projectile(play) {
            return;
        }
        if self.dinolfos() {
            if self.timer2 != 0 {
                self.timer2 -= 1;
                if abs_yaw_towards_player_rel_to_head < 0x1FFE {
                    self.timer2 = 0;
                } else {
                    return;
                }
            } else if self.dinolfos_pick_action(play) {
                return;
            }
        }
        let abs_yaw_diff_with_player = abs16(player.rot_y.wrapping_sub(self.actor.shape_rot.y));
        if self.actor.xz_dist_to_player < 100.0 && player.melee_weapon_state != 0 && abs_yaw_diff_with_player >= 0x1F40 {
            self.actor.shape_rot.y = self.actor.yaw_towards_player;
            self.actor.world_rot.y = self.actor.yaw_towards_player;
            self.setup_circle_player_if_floor(play);
        } else if self.timer != 0 {
            self.timer -= 1;
        } else {
            if self.is_facing_player(0x1555) {
                if self.actor.xz_dist_to_player < 200.0 && self.actor.xz_dist_to_player > 100.0 && play.rand.zero_one() < 0.3 {
                    if self.dinolfos() {
                        self.actor.world_rot.y = self.actor.yaw_towards_player;
                        self.actor.shape_rot.y = self.actor.yaw_towards_player;
                        self.setup_jump_forwards(play);
                    } else {
                        self.setup_circle_player_if_floor(play);
                    }
                } else if play.rand.zero_one() > 0.3 {
                    self.setup_main_walk(play);
                } else {
                    self.setup_circle_player_if_floor(play);
                }
            } else {
                self.setup_turn_towards_player();
            }
            if play.gameplay_frames & 0x5F == 0 {
                audio_play_actor_sfx2(play, NA_SE_EN_RIZA_CRY);
            }
        }
    }

    /// `EnZf_SetupMainWalk`: walking (and choosing what's next); a pair aims for the platform
    /// nearest Link.
    fn setup_main_walk(&mut self, play: &mut PlayState) {
        self.morph_to_loop(|a| &a.walk, -4.0);
        self.action = ENZF_ACTION_MAIN_WALK;
        if self.paired() {
            let ppos = player_view(play).pos;
            self.cur_platform = find_platform(self.actor.world_pos, self.cur_platform);
            self.target_platform = find_platform_close_to_player(self.actor.world_pos, self.cur_platform, self.home_platform, ppos);
            self.action_state = 0;
        }
        self.actor.speed_xz = 0.0;
        self.func = Func::MainWalk;
    }

    /// `EnZf_MainWalk`.
    fn main_walk(&mut self, play: &mut PlayState) {
        let player = player_view(play);
        let mut player_platform: i16 = -1;
        let mut max_target_platform_dist_for_jump = 350.0;
        let mut extra_dist_from_player = 0.0;
        if self.paired() {
            player_platform = find_platform(player.pos, -1);
            self.cur_platform = find_platform(self.actor.world_pos, player_platform);
            if self.actor.world_pos.y >= 420.0 {
                max_target_platform_dist_for_jump = 270.0;
            }
        }
        if self.react_to_projectile(play) {
            return;
        }
        // (May only pass for a pair.)
        if player_platform != self.cur_platform {
            self.target_platform = find_platform_close_to_player(self.actor.world_pos, self.cur_platform, self.home_platform, player.pos);
            if player_platform < 0 && self.target_platform == self.cur_platform {
                player_platform = self.cur_platform;
                self.actor.shape_rot.y = self.actor.yaw_towards_player;
                self.actor.world_rot.y = self.actor.yaw_towards_player;
            } else {
                let target = platform_pos(self.target_platform);
                let yaw = self.yaw_toward_point(target);
                self.actor.shape_rot.y = yaw;
                self.actor.yaw_towards_player = yaw;
                self.actor.world_rot.y = yaw;
                let abs_yaw_diff = abs16(self.actor.wall_yaw.wrapping_sub(self.actor.shape_rot.y));
                if (self.no_floor_ahead && self.actor.speed_xz > 0.0) || (self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 && abs_yaw_diff >= 0x5C19) {
                    if dist_xz(self.actor.world_pos, target) < max_target_platform_dist_for_jump && !self.test_no_floor_ahead(play, 191.9956) {
                        self.setup_jump_forwards(play);
                        if self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 {
                            self.actor.velocity.y = 20.0;
                        }
                        return;
                    } else {
                        self.actor.world_rot.y = self.yaw_toward_point(platform_pos(self.cur_platform));
                    }
                } else {
                    self.actor.shape_rot.y = self.actor.yaw_towards_player;
                    self.actor.world_rot.y = self.actor.yaw_towards_player;
                    smooth_step_to_f(&mut self.actor.speed_xz, 8.0, 1.0, 1.5, 0.0);
                }
            }
        }
        if actor_other_is_locked_on(play, &self.actor) {
            extra_dist_from_player = 100.0;
        }
        if self.actor.xz_dist_to_player <= 70.0 + extra_dist_from_player {
            smooth_step_to_f(&mut self.actor.speed_xz, -8.0, 1.0, 0.5, 0.0);
        } else {
            smooth_step_to_f(&mut self.actor.speed_xz, 8.0, 1.0, 0.5, 0.0);
        }
        self.skel.play_speed = self.actor.speed_xz * 1.2;
        let abs_yaw_diff = abs16(player.rot_y.wrapping_sub(self.actor.shape_rot.y));
        if player_platform == self.cur_platform && self.actor.xz_dist_to_player < 150.0 && player.melee_weapon_state != 0 && abs_yaw_diff >= 0x1F40 {
            self.actor.shape_rot.y = self.actor.yaw_towards_player;
            self.actor.world_rot.y = self.actor.yaw_towards_player;
            if play.rand.zero_one() > 0.7 {
                self.setup_circle_player_if_floor(play);
                return;
            }
        }
        let prev_frame = self.skel.cur_frame as i32;
        self.skel.update();
        let before_cur_frame = (self.skel.cur_frame - self.skel.play_speed.abs()) as i32;
        let abs_play_speed = self.skel.play_speed.abs();
        if player_platform == self.cur_platform {
            if !self.is_facing_player(0x11C7) {
                if play.rand.zero_one() > 0.5 {
                    self.setup_sidestep_if_floor(play);
                } else {
                    self.setup_idle(play);
                }
            } else if self.actor.xz_dist_to_player < 100.0 {
                if play.rand.zero_one() > 0.05 && self.can_attack(play) {
                    self.setup_attack(play);
                } else if play.rand.zero_one() > 0.5 {
                    self.setup_circle_player_if_floor(play);
                } else {
                    self.setup_idle(play);
                }
            } else if self.no_floor_ahead {
                self.setup_sidestep_if_floor(play);
            } else if play.rand.zero_one() < 0.1 {
                self.setup_idle(play);
            }
        }
        if self.dinolfos() {
            if !self.dinolfos_pick_action(play) {
                if self.actor.xz_dist_to_player < 180.0 && self.actor.xz_dist_to_player > 160.0 && self.is_facing_player(0x71C) {
                    if actor_is_locked_on(play, &self.actor) {
                        if play.rand.zero_one() < 0.1 {
                            self.actor.world_rot.y = self.actor.yaw_towards_player;
                            self.actor.shape_rot.y = self.actor.yaw_towards_player;
                            self.setup_jump_forwards(play);
                            return;
                        }
                    } else {
                        self.setup_circle_player_if_floor(play);
                        return;
                    }
                }
            } else {
                return;
            }
        }
        if play.gameplay_frames & 0x5F == 0 {
            audio_play_actor_sfx2(play, NA_SE_EN_RIZA_CRY);
        }
        if prev_frame != self.skel.cur_frame as i32
            && ((before_cur_frame < 2 && abs_play_speed as i32 + prev_frame >= 4) || (before_cur_frame < 32 && abs_play_speed as i32 + prev_frame >= 34))
        {
            audio_play_actor_sfx2(play, NA_SE_EN_RIZA_WALK);
        }
    }

    /// `EnZf_SetupJumpForwards`: 15 up, 16 on (a pair) or 10.
    fn setup_jump_forwards(&mut self, play: &mut PlayState) {
        self.change(|a| &a.jump, 1.0, 0.0, Some(3.0), ANIMMODE_ONCE, -3.0);
        self.timer = 0;
        self.action_state = 1;
        self.actor.velocity.y = 15.0;
        self.actor.speed_xz = if self.paired() { 16.0 } else { 10.0 };
        self.action = ENZF_ACTION_JUMP_FORWARDS;
        audio_play_actor_sfx2(play, NA_SE_EN_RIZA_JUMP);
        self.func = Func::JumpForwards;
    }

    /// `EnZf_JumpForwards`.
    fn jump_forwards(&mut self, play: &mut PlayState) {
        if self.timer != 0 && self.actor.world_pos.y <= self.actor.floor_height {
            self.actor.world_pos.y = self.actor.floor_height;
            self.action_state = 0;
            self.actor.velocity.y = 0.0;
            self.actor.speed_xz = 0.0;
        }
        if self.skel.update() {
            if self.timer == 0 {
                self.change(|a| &a.land_on_feet, 3.0, 0.0, Some(17.0), ANIMMODE_ONCE, -3.0);
                self.timer = 10;
                audio_play_actor_sfx2(play, NA_SE_EN_RIZA_JUMP);
            } else {
                self.actor.speed_xz = 0.0;
                self.action_state = 0;
                self.setup_main_walk(play);
            }
        }
        if play.gameplay_frames & 0x5F == 0 {
            audio_play_actor_sfx2(play, NA_SE_EN_RIZA_CRY);
        }
        if self.dinolfos() && self.actor.bg_check_flags & (BGCHECKFLAG_GROUND | BGCHECKFLAG_GROUND_TOUCH) != 0 {
            if self.can_attack(play) {
                self.setup_attack(play);
            } else {
                self.setup_circle_player_if_floor(play);
            }
        }
    }

    /// `EnZf_SetupTurnTowardsPlayer` (its speed is `EnZf_SetupIdle`'s 0, its only user's).
    fn setup_turn_towards_player(&mut self) {
        self.morph_to_loop(|a| &a.walk, -4.0);
        self.action = ENZF_ACTION_TURN_TOWARDS_PLAYER;
        self.func = Func::TurnTowardsPlayer;
    }

    /// `EnZf_TurnTowardsPlayer`: a quarter of the way plus 2000 a frame.
    fn turn_towards_player(&mut self, play: &mut PlayState) {
        if self.react_to_projectile(play) || (self.dinolfos() && self.dinolfos_pick_action(play)) {
            return;
        }
        let ppos = player_view(play).pos;
        let rel = self.actor.yaw_towards_player.wrapping_sub(self.actor.shape_rot.y);
        let yaw_speed = if rel > 0 { ((rel as f32 * 0.25) + 2000.0) as i32 as i16 } else { ((rel as f32 * 0.25) - 2000.0) as i32 as i16 };
        self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(yaw_speed);
        self.actor.world_rot.y = self.actor.shape_rot.y;
        let mut anim_play_speed = yaw_speed as f32 * 1.5;
        if rel > 0 {
            if anim_play_speed > 2.0 {
                anim_play_speed = 2.0;
            }
        } else if anim_play_speed < -2.0 {
            anim_play_speed = -2.0;
        }
        self.skel.play_speed = -anim_play_speed;
        self.skel.update();
        if self.paired() {
            self.cur_platform = find_platform(self.actor.world_pos, self.cur_platform);
            if find_platform(ppos, -1) != self.cur_platform {
                self.setup_main_walk(play);
                return;
            }
        }
        if self.is_facing_player(0x1555) {
            if play.rand.zero_one() > 0.8 {
                self.setup_sidestep_if_floor(play);
            } else {
                self.setup_main_walk(play);
            }
        }
        if play.gameplay_frames & 0x5F == 0 {
            audio_play_actor_sfx2(play, NA_SE_EN_RIZA_CRY);
        }
    }

    /// `EnZf_SetupSidestepIfFloor`: side-stepping 20 to 29 frames (a pair only with floor 40 to a
    /// side), else the walk.
    fn setup_sidestep_if_floor(&mut self, play: &mut PlayState) {
        let side = self.actor.shape_rot.y.wrapping_add(0x3FFF);
        if !self.paired() || actor_test_floor_in_direction(&mut self.actor, play, 40.0, side) != 0 || actor_test_floor_in_direction(&mut self.actor, play, -40.0, side) != 0 {
            self.play_loop(|a| &a.walk_sideways);
            self.actor.speed_xz = play.rand.centered_float(12.0);
            self.actor.world_rot.y = self.actor.shape_rot.y;
            self.timer = ((play.rand.zero_one() * 10.0) + 20.0) as i32;
            self.action_state = 0;
            self.action = ENZF_ACTION_SIDESTEP;
            self.dodge_speed = 0.0;
            self.func = Func::Sidestep;
        } else {
            self.setup_main_walk(play);
        }
    }

    /// The side-step's and the circling's turn at a wall or an edge (not a pair's: its
    /// `noFloorAhead` turns it back).
    fn bounce_off_walls(&mut self, play: &mut PlayState) {
        let (speed, side) = (self.actor.speed_xz, self.actor.shape_rot.y.wrapping_add(0x3FFF));
        if self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 || actor_test_floor_in_direction(&mut self.actor, play, speed, side) == 0 {
            let yaw = if self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 {
                let dir = if self.actor.speed_xz >= 0.0 { self.actor.shape_rot.y.wrapping_add(0x3FFF) } else { self.actor.shape_rot.y.wrapping_sub(0x3FFF) };
                self.actor.wall_yaw.wrapping_sub(dir)
            } else {
                self.actor.speed_xz *= -0.8;
                0
            };
            if abs_i(yaw) > 0x4000 {
                self.actor.speed_xz *= -0.8;
                if self.actor.speed_xz < 0.0 {
                    self.actor.speed_xz -= 0.5;
                } else {
                    self.actor.speed_xz += 0.5;
                }
            }
        }
    }

    /// The side-step's and the circling's distance kept 70 to 90 from Link (170 to 190 while he's
    /// locked on to another): `dodgeSpeed` along its facing, where there's floor.
    fn keep_distance(&mut self, play: &mut PlayState) {
        let mut extra = 0.0;
        if actor_other_is_locked_on(play, &self.actor) {
            extra = 100.0;
        }
        if self.actor.xz_dist_to_player <= 70.0 + extra {
            smooth_step_to_f(&mut self.dodge_speed, -4.0, 1.0, 1.5, 0.0);
        } else if 90.0 + extra < self.actor.xz_dist_to_player {
            smooth_step_to_f(&mut self.dodge_speed, 4.0, 1.0, 1.5, 0.0);
        } else {
            smooth_step_to_f(&mut self.dodge_speed, 0.0, 1.0, 5.65, 0.0);
        }
        if self.dodge_speed != 0.0 && !self.test_no_floor_ahead_shape(play, self.dodge_speed) {
            self.actor.world_pos.x += sin_s(self.actor.shape_rot.y) * self.dodge_speed;
            self.actor.world_pos.z += cos_s(self.actor.shape_rot.y) * self.dodge_speed;
        }
    }

    /// `EnZf_Sidestep`.
    fn sidestep(&mut self, play: &mut PlayState) {
        let player = player_view(play);
        smooth_step_to_s(&mut self.actor.shape_rot.y, self.actor.yaw_towards_player, 1, 0xFA0, 1);
        if self.react_to_projectile(play) || (self.dinolfos() && self.dinolfos_pick_action(play)) {
            return;
        }
        self.actor.world_rot.y = self.actor.shape_rot.y.wrapping_add(0x3A98);
        let player_yaw_opposite = player.rot_y.wrapping_add(-0x8000i32 as i16);
        if sin_s(player_yaw_opposite.wrapping_sub(self.actor.shape_rot.y)) >= 0.0 {
            self.actor.speed_xz -= 0.25;
            if self.actor.speed_xz < -8.0 {
                self.actor.speed_xz = -8.0;
            }
        } else if sin_s(player_yaw_opposite.wrapping_sub(self.actor.shape_rot.y)) < 0.0 {
            self.actor.speed_xz += 0.25;
            if self.actor.speed_xz > 8.0 {
                self.actor.speed_xz = 8.0;
            }
        }
        if self.paired() {
            if self.no_floor_ahead {
                self.actor.speed_xz = -self.actor.speed_xz;
            }
        } else {
            self.bounce_off_walls(play);
        }
        self.keep_distance(play);
        if self.actor.speed_xz.abs() >= self.dodge_speed.abs() {
            self.skel.play_speed = self.actor.speed_xz * 0.75;
        } else if self.skel.play_speed < 0.0 {
            self.skel.play_speed = self.dodge_speed * -0.75;
        } else {
            self.skel.play_speed = self.dodge_speed * 0.75;
        }
        let prev_frame = self.skel.cur_frame as i32;
        self.skel.update();
        let before_cur_frame = (self.skel.cur_frame - self.skel.play_speed.abs()) as i32;
        let abs_play_speed = self.skel.play_speed.abs();
        if prev_frame != self.skel.cur_frame as i32
            && ((before_cur_frame < 0xE && abs_play_speed as i32 + prev_frame >= 0x10) || (before_cur_frame < 0x1B && abs_play_speed as i32 + prev_frame >= 0x1D))
        {
            audio_play_actor_sfx2(play, NA_SE_EN_RIZA_WALK);
        }
        if play.gameplay_frames & 0x5F == 0 {
            audio_play_actor_sfx2(play, NA_SE_EN_RIZA_CRY);
        }
        if cos_s(player_yaw_opposite.wrapping_sub(self.actor.shape_rot.y)) < -0.85 || self.timer == 0 {
            self.actor.world_rot.y = self.actor.shape_rot.y;
            if self.actor.xz_dist_to_player <= 100.0 && play.gameplay_frames % 4 == 0 && self.can_attack(play) {
                self.setup_attack(play);
            } else {
                self.setup_idle(play);
            }
        } else if self.timer != 0 {
            self.timer -= 1;
        }
    }

    /// `EnZf_SetupAttack`: the slash at 1.25 (the Dinolfos 1.75).
    fn setup_attack(&mut self, play: &mut PlayState) {
        self.change(|a| &a.slash, 1.25, 0.0, None, ANIMMODE_ONCE, -4.0);
        if self.dinolfos() {
            self.skel.play_speed = 1.75;
        }
        self.sword_collider.base.at_flags &= !AT_BOUNCED;
        self.action = ENZF_ACTION_ATTACK;
        audio_play_actor_sfx2(play, NA_SE_EN_RIZA_CRY);
        self.actor.speed_xz = 0.0;
        self.func = Func::Attack;
    }

    /// `EnZf_Attack`: `NA_SE_EN_RIZA_ATTACK` at frame 10; at its end, the trail's gap, then idle
    /// (by chance, or Link 120 away), round him, or another slash.
    fn attack(&mut self, play: &mut PlayState) {
        let player = player_view(play);
        self.actor.speed_xz = 0.0;
        if self.skel.cur_frame as i32 == 10 {
            audio_play_actor_sfx2(play, NA_SE_EN_RIZA_ATTACK);
        }
        if self.skel.update() {
            self.add_blure_space(play);
            if self.dinolfos() && !self.is_facing_player(0x1554) {
                self.setup_idle(play);
                self.timer = ((play.rand.zero_one() * 5.0) + 5.0) as i32;
                self.timer2 = ((play.rand.zero_one() * 20.0) + 100.0) as i32 as i16;
            } else if play.rand.zero_one() > 0.7 || self.actor.xz_dist_to_player >= 120.0 {
                self.setup_idle(play);
                self.timer = ((play.rand.zero_one() * 5.0) + 5.0) as i32;
            } else {
                self.actor.world_rot.y = self.actor.yaw_towards_player;
                if play.rand.zero_one() > 0.7 {
                    self.setup_circle_player_if_floor(play);
                } else {
                    let abs_yaw_diff_with_player = abs16(player.rot_y.wrapping_sub(self.actor.shape_rot.y));
                    if abs_yaw_diff_with_player <= 0x2710 {
                        let abs_rel_yaw_towards_player = abs16(self.actor.yaw_towards_player.wrapping_sub(self.actor.shape_rot.y));
                        if abs_rel_yaw_towards_player > 0x3E80 {
                            self.actor.world_rot.y = self.actor.yaw_towards_player;
                            self.setup_circle_player_if_floor(play);
                        } else if player.state1 & (player::STATE1_4 | player::STATE1_13 | player::STATE1_14) != 0 {
                            if self.actor.is_targeted {
                                self.setup_attack(play);
                            } else {
                                self.setup_circle_player_if_floor(play);
                            }
                        } else {
                            self.setup_attack(play);
                        }
                    } else {
                        self.setup_circle_player_if_floor(play);
                    }
                }
            }
        }
    }

    /// `EnZf_SetupAttackBounced`: the slash backwards from 3 frames back.
    fn setup_attack_bounced(&mut self) {
        let f = self.skel.cur_frame - 3.0;
        self.change(|a| &a.slash, -1.0, f, Some(0.0), ANIMMODE_ONCE, 0.0);
        self.action = ENZF_ACTION_ATTACK_BOUNCED;
        self.func = Func::AttackBounced;
    }

    /// `EnZf_AttackBounced`.
    fn attack_bounced(&mut self, play: &mut PlayState) {
        if self.skel.update() {
            if play.rand.zero_one() > 0.7 {
                self.setup_idle(play);
            } else if play.rand.zero_one() > 0.2 && self.can_attack(play) {
                self.setup_attack(play);
            } else {
                self.setup_circle_player_if_floor(play);
            }
        }
    }

    /// `EnZf_SetupJumpBackwards`: 15 up, 15 back.
    fn setup_jump_backwards(&mut self, play: &mut PlayState) {
        self.change(|a| &a.jump, -1.0, 3.0, Some(0.0), ANIMMODE_ONCE, -3.0);
        self.timer = 0;
        self.action_state = 1;
        self.action = ENZF_ACTION_JUMP_BACKWARDS;
        self.actor.velocity.y = 15.0;
        self.actor.speed_xz = -15.0;
        audio_play_actor_sfx2(play, NA_SE_EN_RIZA_JUMP);
        self.func = Func::JumpBackwards;
    }

    /// `EnZf_JumpBackwards`.
    fn jump_backwards(&mut self, play: &mut PlayState) {
        if self.timer != 0 && self.actor.world_pos.y <= self.actor.floor_height {
            self.actor.world_pos.y = self.actor.floor_height;
            self.action_state = 0;
            self.actor.velocity.y = 0.0;
            self.actor.speed_xz = 0.0;
        }
        if self.skel.update() {
            if self.timer == 0 {
                self.change(|a| &a.land_on_feet, 3.0, 0.0, Some(17.0), ANIMMODE_ONCE, -3.0);
                self.timer = 10;
                audio_play_actor_sfx2(play, NA_SE_EN_RIZA_JUMP);
            } else if play.gameplay_frames % 2 != 0 {
                self.setup_circle_player_if_floor(play);
            } else {
                self.setup_idle(play);
            }
        }
        if play.state_frames & 0x5F == 0 {
            audio_play_actor_sfx2(play, NA_SE_EN_RIZA_CRY);
        }
    }

    /// `EnZf_SetupStunned`: held at the hurt animation's first frame (ice: 36 frames of chunks).
    fn setup_stunned(&mut self, play: &mut PlayState) {
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 && (self.actor.velocity.y == 0.0 || self.actor.velocity.y == -4.0) {
            self.actor.speed_xz = 0.0;
            self.action_state = 0;
        } else {
            self.action_state = 1;
        }
        if self.last_damage_reaction == ENZF_DMG_REACT_ICE {
            self.frozen_timer = 36;
        } else {
            self.play_once_set_speed(|a| &a.damaged, 0.0);
        }
        audio_play_actor_sfx2(play, NA_SE_EN_GOMA_JR_FREEZE);
        self.action = ENZF_ACTION_STUNNED;
        self.func = Func::Stunned;
    }

    /// `EnZf_Stunned`: until its colour's gone on the ground: dead, or a pick.
    fn stunned(&mut self, play: &mut PlayState) {
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND_TOUCH != 0 {
            self.actor.speed_xz = 0.0;
        }
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
            if self.actor.speed_xz < 0.0 {
                self.actor.speed_xz += 0.05;
            }
            self.action_state = 0;
        }
        if self.actor.color_filter_timer == 0 && self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
            if self.actor.col_chk_info.health == 0 {
                self.setup_die(play);
            } else if !(self.dinolfos() && self.dinolfos_pick_action(play)) {
                if inactive_type(play) != ENZF_TYPE_LIZALFOS_SINGLE {
                    self.pick_action(play);
                } else {
                    let abs_rel_wall_yaw = abs16(self.actor.wall_yaw.wrapping_sub(self.actor.shape_rot.y));
                    if self.dinolfos() && self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 && abs_i(abs_rel_wall_yaw) < 0x2EE0 && self.actor.xz_dist_to_player < 90.0 {
                        self.actor.world_rot.y = self.actor.shape_rot.y;
                        self.setup_jump_attack(play);
                    } else if !self.react_to_projectile(play) {
                        if !self.dinolfos() {
                            self.pick_action(play);
                        } else if self.actor.xz_dist_to_player <= 100.0 && play.gameplay_frames % 4 != 0 && self.can_attack(play) {
                            self.setup_attack(play);
                        } else {
                            self.pick_action(play);
                        }
                    }
                }
            }
        }
    }

    /// `EnZf_Paired_SetupJumpAwayStartTurnAround`: turning away from Link (twice the speed) towards
    /// the platform to jump to.
    fn paired_setup_jump_away_start_turn_around(&mut self, play: &mut PlayState) {
        let ppos = player_view(play).pos;
        let mut anim_morph_frames = 0.0;
        let anim_end_frame = self.last_frame(|a| &a.run_away_turn_around);
        if self.action <= ENZF_ACTION_DAMAGED {
            anim_morph_frames = -4.0;
        }
        self.change(|a| &a.run_away_turn_around, 2.0, 0.0, Some(anim_end_frame), ANIMMODE_ONCE, anim_morph_frames);
        self.action = ENZF_ACTION_PAIRED_JUMP_AWAY_START_TURN_AROUND;
        self.actor.speed_xz = 0.0;
        self.cur_platform = find_platform(self.actor.world_pos, self.cur_platform);
        self.target_platform = find_platform_without_player(self.actor.world_pos, self.cur_platform, self.home_platform, ppos);
        self.actor.world_rot.y = self.actor.shape_rot.y;
        self.func = Func::PairedJumpAwayStartTurnAround;
    }

    /// `EnZf_Paired_JumpAwayStartTurnAround`: its back to the target platform; at the end it jumps
    /// away, its sword sheathed.
    fn paired_jump_away_start_turn_around(&mut self, play: &mut PlayState) {
        let yaw = self.yaw_toward_point(platform_pos(self.target_platform)).wrapping_add(-0x8000i32 as i16);
        smooth_step_to_s(&mut self.actor.world_rot.y, yaw, 1, 0x3E8, 0);
        self.actor.shape_rot.y = self.actor.world_rot.y;
        if self.skel.update() {
            self.actor.world_rot.y = yaw.wrapping_sub(-0x8000i32 as i16);
            self.paired_setup_jump_away(play);
            self.is_sword_sheathed = true;
        }
    }

    /// `EnZf_Paired_SetupIdleHop`: hopping in place until signalled to engage (`timer2`,
    /// `EnZf_Update`), jumping away from Link coming near.
    fn paired_setup_idle_hop(&mut self) {
        self.action_state = 0;
        self.change(Self::run_away_anim(0), 1.0, 0.0, None, ANIMMODE_ONCE, -4.0);
        self.action = ENZF_ACTION_PAIRED_IDLE_HOP;
        self.actor.speed_xz = 0.0;
        self.dodge_velocity_y = 0.0;
        self.dodge_speed = 0.0;
        self.func = Func::PairedIdleHop;
    }

    /// `EnZf_Paired_IdleHop`: facing away from Link; within 400 (250 on the upper floor) it jumps
    /// away; else hops (the run-away jump's three parts in turn, 10 up), dodging by its hop.
    fn paired_idle_hop(&mut self, play: &mut PlayState) {
        let mut min_player_dist = 400.0;
        smooth_step_to_f(&mut self.actor.speed_xz, 0.0, 1.0, 0.5, 0.0);
        smooth_step_to_s(&mut self.actor.shape_rot.y, self.actor.yaw_towards_player.wrapping_add(-0x8000i32 as i16), 1, 0xFA0, 0);
        if self.actor.world_pos.y >= 420.0 {
            min_player_dist = 250.0;
        }
        if self.actor.xz_dist_to_player < min_player_dist && self.action_state != 1 {
            self.actor.shape_rot.y = self.actor.world_rot.y;
            self.paired_setup_jump_away_start_turn_around(play);
            return;
        }
        if self.action_state != 1 {
            self.react_to_projectile_set_dodge_speed(play);
        }
        if self.skel.update() {
            self.action_state += 1;
            if self.action_state >= 3 {
                self.action_state = 0;
            }
            if self.dodge_speed != 0.0 || self.dodge_velocity_y != 0.0 {
                self.action_state = 1;
            }
            let mut anim_end_frame = self.last_frame(Self::run_away_anim(self.action_state));
            match self.action_state {
                0 => {
                    self.actor.velocity.y = 0.0;
                    self.actor.world_pos.y = self.actor.floor_height;
                }
                1 => {
                    self.actor.velocity.y = self.dodge_velocity_y + 10.0;
                    self.actor.speed_xz = self.dodge_speed;
                    self.dodge_speed = 0.0;
                    self.dodge_velocity_y = 0.0;
                }
                2 => {
                    anim_end_frame = 3.0;
                    self.actor.world_pos.y = self.actor.floor_height;
                }
                _ => {}
            }
            self.change(Self::run_away_anim(self.action_state), 1.5, 0.0, Some(anim_end_frame), ANIMMODE_ONCE, 0.0);
        }
        if play.gameplay_frames & 0x5F == 0 {
            audio_play_actor_sfx2(play, NA_SE_EN_RIZA_CRY);
        }
    }

    /// `EnZf_Paired_SetupJumpAway`: away from Link, to a platform without him.
    fn paired_setup_jump_away(&mut self, play: &mut PlayState) {
        let ppos = player_view(play).pos;
        self.action_state = 0;
        if let Some(a) = self.anim(Self::run_away_anim(0)) {
            self.skel.play_once(a);
        }
        self.action = ENZF_ACTION_PAIRED_JUMP_AWAY;
        self.cur_platform = find_platform(self.actor.world_pos, self.cur_platform);
        self.target_platform = find_platform_without_player(self.actor.world_pos, self.cur_platform, self.home_platform, ppos);
        self.func = Func::PairedJumpAway;
    }

    /// `EnZf_Paired_JumpAway`: back first (facing away from where it goes), a jump as far as the
    /// floor ahead needs (107 or 220 on, else the first floor 117 to 317 on, else hopping), steered
    /// to the platform in the air; landed 550 (280 above) from Link it hops, else at the platform it
    /// picks the next.
    fn paired_jump_away(&mut self, play: &mut PlayState) {
        let ppos = player_view(play).pos;
        let mut anim_play_speed = 1.0;
        let mut var_fs0 = 550.0;
        let target = platform_pos(self.target_platform);
        let dist_to_target_platform = dist_xz(self.actor.world_pos, target);
        let prev_action_state = self.action_state;
        if play.gameplay_frames & 0x5F == 0 {
            audio_play_actor_sfx2(play, NA_SE_EN_RIZA_CRY);
        }
        if self.actor.world_pos.y >= 420.0 {
            var_fs0 = 280.0;
        }
        let yaw_towards_target_platform = self.yaw_toward_point(target);
        match self.action_state {
            0 => {
                self.actor.world_rot.y = yaw_towards_target_platform;
                self.actor.shape_rot.y = yaw_towards_target_platform.wrapping_add(-0x8000i32 as i16);
                statics(play).d_80b4ab30 = 0;
                self.home_platform = self.cur_platform;
                let mut floor_ahead_flags = 0;
                floor_ahead_flags |= (!self.test_no_floor_ahead(play, 107.0) as i32) << 0;
                floor_ahead_flags |= (!self.test_no_floor_ahead(play, 220.0) as i32) << 1;
                self.action_state += 1;
                match floor_ahead_flags {
                    1 | 3 => {
                        self.actor.velocity.y = 12.0;
                        if self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 {
                            self.actor.velocity.y += 8.0;
                        }
                        self.actor.speed_xz = 8.0;
                    }
                    2 => {
                        self.actor.velocity.y = 15.0;
                        self.actor.speed_xz = 20.0;
                    }
                    _ => {
                        let mut candidate_distance: f32 = 107.0;
                        candidate_distance += 10.0;
                        let mut candidate_speed: f32 = 8.0;
                        candidate_speed += 1.2;
                        let mut i = 20;
                        while i >= 0 {
                            if !self.test_no_floor_ahead(play, candidate_distance) {
                                self.actor.speed_xz = candidate_speed;
                                self.actor.velocity.y = 12.0;
                                break;
                            }
                            i -= 1;
                            candidate_distance += 10.0;
                            candidate_speed += 1.2;
                        }
                        if self.actor.speed_xz == 0.0 {
                            self.paired_setup_idle_hop();
                        }
                    }
                }
            }
            1 => {
                if self.actor.bg_check_flags & BGCHECKFLAG_GROUND_TOUCH != 0 || self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
                    audio_play_actor_sfx2(play, NA_SE_EN_RIZA_ONGND);
                    self.actor.velocity.y = 0.0;
                    self.actor.world_pos.y = self.actor.floor_height;
                    self.actor.speed_xz = 0.0;
                    self.feet_dust(play);
                    if self.actor.xz_dist_to_player >= var_fs0 {
                        self.paired_setup_idle_hop();
                    } else if dist_to_target_platform < 80.0 {
                        self.cur_platform = find_platform(self.actor.world_pos, self.cur_platform);
                        self.target_platform = find_platform_without_player(self.actor.world_pos, self.cur_platform, self.home_platform, ppos);
                    }
                    self.action_state = 0;
                    anim_play_speed = 2.0;
                } else {
                    smooth_step_to_s(&mut self.actor.world_rot.y, yaw_towards_target_platform, 1, 0xFA0, 0);
                    self.actor.shape_rot.y = self.actor.world_rot.y.wrapping_add(-0x8000i32 as i16);
                    statics(play).d_80b4ab30 += 1;
                }
            }
            2 => {
                if self.skel.cur_frame == self.skel.end_frame {
                    self.action_state = 0;
                }
            }
            _ => {}
        }
        if prev_action_state != self.action_state {
            self.play_once_set_speed(Self::run_away_anim(self.action_state), anim_play_speed);
        }
        self.skel.update();
    }

    /// `EnZf_Paired_SetupEngageCombat`: turning round and drawing its sword.
    fn paired_setup_engage_combat(&mut self, play: &mut PlayState) {
        let ppos = player_view(play).pos;
        if let Some(a) = self.anim(|a| &a.turn_around_unsheath_sword) {
            self.skel.play_once(a);
        }
        self.actor.world_rot.y = self.actor.world_rot.y.wrapping_add(-0x8000i32 as i16);
        self.action = ENZF_ACTION_PAIRED_ENGAGE_COMBAT;
        self.actor.speed_xz = 0.0;
        self.cur_platform = find_platform(self.actor.world_pos, self.cur_platform);
        self.target_platform = find_platform_without_player(self.actor.world_pos, self.cur_platform, self.home_platform, ppos);
        self.func = Func::PairedEngageCombat;
    }

    /// `EnZf_Paired_EngageCombat`: turning to Link from frame 26, the sword out at frame 22, then
    /// idle (`NA_SE_EN_RIZA_CRY`).
    fn paired_engage_combat(&mut self, play: &mut PlayState) {
        let yaw_towards_player = self.actor.yaw_towards_player;
        if self.skel.cur_frame >= 26.0 {
            smooth_step_to_s(&mut self.actor.shape_rot.y, yaw_towards_player, 1, 0x1770, 0);
        }
        if self.skel.update() {
            audio_play_actor_sfx2(play, NA_SE_EN_RIZA_CRY);
            self.actor.world_rot.y = yaw_towards_player;
            self.action_state = -1;
            self.setup_idle(play);
        }
        if self.skel.cur_frame == 22.0 {
            self.is_sword_sheathed = false;
        }
    }

    /// `EnZf_SetupDamaged`: knocked back 4 on the ground (`NA_SE_EN_RIZA_DAMAGE`).
    fn setup_damaged(&mut self, play: &mut PlayState) {
        self.change(|a| &a.damaged, 1.5, 0.0, None, ANIMMODE_ONCE, -4.0);
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 && (self.actor.velocity.y == 0.0 || self.actor.velocity.y == -4.0) {
            self.actor.speed_xz = -4.0;
            self.action_state = 0;
        } else {
            self.action_state = 1;
        }
        if self.dinolfos() {
            self.skel.play_speed = 4.5;
        }
        if !self.paired() {
            self.actor.world_rot.y = self.actor.yaw_towards_player;
        }
        audio_play_actor_sfx2(play, NA_SE_EN_RIZA_DAMAGE);
        self.action = ENZF_ACTION_DAMAGED;
        self.func = Func::Damaged;
    }

    /// `EnZf_Damaged`: at the hurt animation's end on the ground: a pair's fighter that's to flee
    /// signals it (`sPairedActiveFleeFlag`); else a jump attack, a slash or a pick.
    fn damaged(&mut self, play: &mut PlayState) {
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND_TOUCH != 0 {
            self.actor.speed_xz = 0.0;
        }
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
            if self.actor.speed_xz < 0.0 {
                self.actor.speed_xz += 0.05;
            }
            self.action_state = 0;
        }
        smooth_step_to_s(&mut self.actor.shape_rot.y, self.actor.yaw_towards_player, 1, 0x1194, 0);
        if (self.dinolfos() && self.dinolfos_pick_action(play)) || !self.skel.update() || self.actor.bg_check_flags & BGCHECKFLAG_GROUND == 0 {
            return;
        }
        if inactive_type(play) != ENZF_TYPE_LIZALFOS_SINGLE {
            if self.last_damage_reaction == ENZF_DMG_REACT_FLEE {
                let s = statics(play);
                s.s_paired_active_flee_flag = s.s_paired_active_flee_flag.wrapping_add(1);
            } else {
                self.actor.world_rot.y = self.actor.shape_rot.y;
                if !self.test_no_floor_ahead(play, 135.0) && self.actor.xz_dist_to_player < 90.0 {
                    self.setup_jump_attack(play);
                } else if self.actor.xz_dist_to_player <= 100.0 && play.gameplay_frames % 4 == 0 {
                    self.setup_attack(play);
                } else {
                    self.pick_action(play);
                }
            }
        } else {
            let abs_rel_wall_yaw = abs16(self.actor.wall_yaw.wrapping_sub(self.actor.shape_rot.y));
            if self.dinolfos() && self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 && abs_i(abs_rel_wall_yaw) < 0x2EE0 && self.actor.xz_dist_to_player < 90.0 {
                self.setup_jump_attack(play);
            } else if !self.react_to_projectile(play) {
                if !self.dinolfos() {
                    self.actor.world_rot.y = self.actor.shape_rot.y;
                    if !self.test_no_floor_ahead(play, 135.0) && self.actor.xz_dist_to_player < 90.0 {
                        self.setup_jump_attack(play);
                    } else if self.actor.xz_dist_to_player <= 100.0 && play.gameplay_frames % 4 == 0 {
                        self.setup_attack(play);
                    } else {
                        self.pick_action(play);
                    }
                } else if self.actor.xz_dist_to_player <= 100.0 && play.gameplay_frames % 4 == 0 && self.can_attack(play) {
                    self.setup_attack(play);
                } else {
                    self.pick_action(play);
                }
            }
        }
    }

    /// `EnZf_SetupJumpAttack`: 22 up, 7.5 on.
    fn setup_jump_attack(&mut self, play: &mut PlayState) {
        self.change(|a| &a.jump, 1.0, 0.0, Some(3.0), ANIMMODE_ONCE, 0.0);
        self.timer = 0;
        self.action_state = 1;
        self.action = ENZF_ACTION_JUMP_ATTACK;
        self.actor.velocity.y = 22.0;
        self.actor.speed_xz = 7.5;
        audio_play_actor_sfx2(play, NA_SE_EN_RIZA_JUMP);
        self.actor.world_rot.y = self.actor.shape_rot.y;
        self.func = Func::JumpAttack;
    }

    /// `EnZf_JumpAttack`: sparkles at its feet while rising; the slash's start in the air, its
    /// end landed (from frame 13, `NA_SE_EN_RIZA_ATTACK`).
    fn jump_attack(&mut self, play: &mut PlayState) {
        smooth_step_to_s(&mut self.actor.shape_rot.y, self.actor.yaw_towards_player, 1, 0xFA0, 1);
        if self.actor.velocity.y >= 5.0 {
            func_800355b8(play, self.left_foot_pos);
            func_800355b8(play, self.right_foot_pos);
        }
        if self.skel.update() {
            if self.timer == 0 {
                self.change(|a| &a.slash, 3.0, 0.0, Some(13.0), ANIMMODE_ONCE, -4.0);
                self.timer = 10;
            } else if self.actor.bg_check_flags & (BGCHECKFLAG_GROUND | BGCHECKFLAG_GROUND_TOUCH) != 0 {
                self.actor.world_rot.y = self.actor.yaw_towards_player;
                self.actor.shape_rot.y = self.actor.yaw_towards_player;
                self.actor.velocity.y = 0.0;
                self.actor.speed_xz = 0.0;
                self.actor.world_pos.y = self.actor.floor_height;
                self.setup_attack(play);
                audio_play_actor_sfx2(play, NA_SE_EN_RIZA_ATTACK);
                self.skel.cur_frame = 13.0;
            }
        }
    }

    /// `EnZf_SetupCirclePlayerIfFloor`: side-stepping round Link 5 to 14 frames, at 6 the way his
    /// facing turns from it (a pair only with floor 40 to a side), else the walk.
    fn setup_circle_player_if_floor(&mut self, play: &mut PlayState) {
        let side = self.actor.shape_rot.y.wrapping_add(0x3FFF);
        if !self.paired() || actor_test_floor_in_direction(&mut self.actor, play, 40.0, side) != 0 || actor_test_floor_in_direction(&mut self.actor, play, -40.0, side) != 0 {
            self.play_loop(|a| &a.walk_sideways);
            let player_yaw = player_view(play).rot_y;
            smooth_step_to_s(&mut self.actor.shape_rot.y, self.actor.yaw_towards_player, 1, 0xFA0, 1);
            if sin_s(player_yaw.wrapping_sub(self.actor.shape_rot.y)) >= 0.0 {
                self.actor.speed_xz = -6.0;
            } else if sin_s(player_yaw.wrapping_sub(self.actor.shape_rot.y)) < 0.0 {
                self.actor.speed_xz = 6.0;
            }
            self.dodge_speed = 0.0;
            self.action_state = 0;
            self.actor.world_rot.y = self.actor.shape_rot.y.wrapping_add(0x3FFF);
            self.timer = ((play.rand.zero_one() * 10.0) + 5.0) as i32;
            self.action = ENZF_ACTION_CIRCLE_PLAYER;
            self.func = Func::CirclePlayer;
        } else {
            self.setup_main_walk(play);
        }
    }

    /// `EnZf_CirclePlayer`: side-stepping round Link, faster the way his facing turns; off his
    /// platform a pair's waiting one hops, the rest walk; at the end idle (Link turned from it), a
    /// slash within 100, a jump at him from 240 to 280, or the walk.
    fn circle_player(&mut self, play: &mut PlayState) {
        let player = player_view(play);
        let mut extra_radius = 0.0;
        smooth_step_to_s(&mut self.actor.shape_rot.y, self.actor.yaw_towards_player, 1, 0xBB8, 1);
        let player_yaw = player.rot_y;
        if self.paired() {
            if self.no_floor_ahead {
                self.actor.speed_xz = -self.actor.speed_xz;
            }
        } else {
            self.bounce_off_walls(play);
        }
        if sin_s(player_yaw.wrapping_sub(self.actor.shape_rot.y)) >= 0.0 {
            self.actor.speed_xz += 0.125;
        } else {
            self.actor.speed_xz -= 0.125;
        }
        self.actor.world_rot.y = self.actor.shape_rot.y.wrapping_add(0x4000);
        if actor_other_is_locked_on(play, &self.actor) {
            extra_radius = 100.0;
        }
        if self.actor.xz_dist_to_player <= 70.0 + extra_radius {
            smooth_step_to_f(&mut self.dodge_speed, -4.0, 1.0, 1.5, 0.0);
        } else if 90.0 + extra_radius < self.actor.xz_dist_to_player {
            smooth_step_to_f(&mut self.dodge_speed, 4.0, 1.0, 1.5, 0.0);
        } else {
            smooth_step_to_f(&mut self.dodge_speed, 0.0, 1.0, 5.65, 0.0);
        }
        if self.dodge_speed != 0.0 && !self.test_no_floor_ahead_shape(play, self.dodge_speed) {
            self.actor.world_pos.x += sin_s(self.actor.shape_rot.y) * self.dodge_speed;
            self.actor.world_pos.z += cos_s(self.actor.shape_rot.y) * self.dodge_speed;
        }
        if self.actor.speed_xz.abs() >= self.dodge_speed.abs() {
            self.skel.play_speed = -self.actor.speed_xz * 0.75;
        } else if self.skel.play_speed < 0.0 {
            self.skel.play_speed = self.dodge_speed * -0.75;
        } else {
            self.skel.play_speed = self.dodge_speed * 0.75;
        }
        let prev_frame = self.skel.cur_frame as i32;
        self.skel.update();
        let before_cur_frame = (self.skel.cur_frame - self.skel.play_speed.abs()) as i32;
        let abs_play_speed = self.skel.play_speed.abs();
        self.cur_platform = find_platform(self.actor.world_pos, self.cur_platform);
        let waiting = self.paired() && inactive_type(play) == self.ty();
        if find_platform(player.pos, -1) != self.cur_platform {
            self.actor.speed_xz = 0.0;
            if waiting {
                self.paired_setup_idle_hop();
            } else {
                self.setup_main_walk(play);
            }
        } else if !(self.dinolfos() && self.dinolfos_pick_action(play)) {
            if self.timer == 0 {
                let yaw = abs16(player.rot_y.wrapping_sub(self.actor.shape_rot.y));
                if yaw >= 0x3A98 {
                    if waiting {
                        self.paired_setup_idle_hop();
                    } else {
                        self.setup_idle(play);
                        self.timer = ((play.rand.zero_one() * 5.0) + 1.0) as i32;
                    }
                } else if waiting {
                    self.paired_setup_idle_hop();
                } else {
                    self.actor.world_rot.y = self.actor.shape_rot.y;
                    if self.actor.xz_dist_to_player <= 100.0 && play.gameplay_frames % 4 == 0 && self.can_attack(play) {
                        self.setup_attack(play);
                    } else if self.actor.xz_dist_to_player < 280.0 && self.actor.xz_dist_to_player > 240.0 && !self.test_no_floor_ahead(play, 191.9956) && play.gameplay_frames % 2 == 0 {
                        self.setup_jump_forwards(play);
                    } else {
                        self.setup_main_walk(play);
                    }
                }
            } else {
                self.timer -= 1;
            }
            if prev_frame != self.skel.cur_frame as i32
                && ((before_cur_frame < 14 && abs_play_speed as i32 + prev_frame >= 16) || (before_cur_frame < 27 && abs_play_speed as i32 + prev_frame >= 29))
            {
                audio_play_actor_sfx2(play, NA_SE_EN_RIZA_WALK);
            }
            if play.gameplay_frames & 0x5F == 0 {
                audio_play_actor_sfx2(play, NA_SE_EN_RIZA_CRY);
            }
        }
    }

    /// `EnZf_SetupDie`: not targetable, `NA_SE_EN_RIZA_DEAD`; the first of a pair to die signals
    /// the other (its health back up to 3).
    fn setup_die(&mut self, play: &mut PlayState) {
        self.change(|a| &a.die, 1.5, 0.0, None, ANIMMODE_ONCE, -4.0);
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 && (self.actor.velocity.y == 0.0 || self.actor.velocity.y == -4.0) {
            self.actor.speed_xz = 0.0;
            self.action_state = 0;
        } else {
            self.action_state = 1;
        }
        self.action = ENZF_ACTION_DIE;
        self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        if inactive_type(play) != ENZF_TYPE_LIZALFOS_SINGLE {
            // Signal the other Lizalfos to engage combat. @bug (game): `prev` (else `next`) is
            // whatever actor is next to it in the enemies' list (NULL `next`: a crash in the C).
            let (prev, next) = self.prev_next(play);
            match prev.or(next) {
                Some(other) => {
                    Self::signal_engage(play, other);
                    if let Some(o) = play.actors.actor_mut(other)
                        && o.col_chk_info.health < 3
                    {
                        o.col_chk_info.health = 3;
                    }
                }
                None => log::debug!("En_Zf: no actor next to it to signal (a NULL access in the C)"),
            }
        }
        statics(play).s_paired_active_flee_flag = 0;
        audio_play_actor_sfx2(play, NA_SE_EN_RIZA_DEAD);
        self.func = Func::Die;
    }

    /// `EnZf_Die`: at the animation's end a prop (the second of a pair: its switch flag, the music
    /// back), then fading 5 a frame and gone; falls at frames 10 and 18 (`NA_SE_EN_RIZA_DOWN`).
    fn die(&mut self, play: &mut PlayState) {
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND_TOUCH != 0 {
            self.actor.speed_xz = 0.0;
        }
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
            smooth_step_to_f(&mut self.actor.speed_xz, 0.0, 1.0, 0.15, 0.0);
            self.action_state = 0;
        }
        if self.skel.update() {
            if self.actor.category != ACTORCAT_PROP {
                if self.paired() && inactive_type(play) == ENZF_TYPE_LIZALFOS_SINGLE {
                    play.flags.set_switch(self.switch_flag as i32);
                    play.audio.func_800f5b58();
                } else {
                    statics(play).s_paired_inactive_type = ENZF_TYPE_LIZALFOS_SINGLE;
                }
                // Actor_ChangeCategory(play, &play->actorCtx, &this->actor, ACTORCAT_PROP).
                self.actor.category = ACTORCAT_PROP;
                if let Some(me) = play.cur_actor {
                    play.actors.change_category(me, ACTORCAT_PROP);
                }
            }
            if self.alpha != 0 {
                self.alpha = self.alpha.wrapping_sub(5);
                self.shadow_alpha = self.alpha;
            } else {
                self.actor.kill();
            }
        } else {
            let temp_ft1 = self.skel.cur_frame as i32;
            if temp_ft1 == 10 || temp_ft1 == 18 {
                audio_play_actor_sfx2(play, NA_SE_EN_RIZA_DOWN);
            }
        }
    }

    /// `EnZf_UpdateHeadRot`: the Dinolfos shakes its head idle after a slash (`timer2`); else it
    /// turns to Link, 0x7D0 a frame, to 0x1CD7.
    fn update_head_rot(&mut self) {
        if self.dinolfos() && self.action == ENZF_ACTION_IDLE && self.timer2 != 0 {
            self.head_rot = (sin_s((self.timer2 as i32 * 0x578) as i16) * 10920.0) as i32 as i16;
        } else {
            let yaw_towards_player_rel_to_head = self.actor.yaw_towards_player.wrapping_sub(self.head_rot.wrapping_add(self.actor.shape_rot.y));
            self.head_rot_step = yaw_towards_player_rel_to_head.clamp(-0x7D0, 0x7D0);
            self.head_rot = self.head_rot.wrapping_add(self.head_rot_step);
            self.head_rot = self.head_rot.clamp(-0x1CD7, 0x1CD7);
        }
    }

    /// `EnZf_CheckCollide`: a hit (up to `ENZF_ACTION_STUNNED`; not the pair's waiting one, not
    /// fire or light magic): stunned (blue 80; ice too), or red 8 and hurt (a pair's fighter from
    /// 4 or more to under 4 is to flee), or dead (a drop from table 4, 14 for the Dinolfos).
    fn check_collide(&mut self, play: &mut PlayState) {
        if self.body_collider.base.ac_flags & AC_HIT == 0 || self.action > ENZF_ACTION_STUNNED {
            return;
        }
        self.body_collider.base.ac_flags &= !AC_HIT;
        let waiting = self.paired() && inactive_type(play) == self.ty();
        let reaction = self.actor.col_chk_info.damage_reaction;
        if waiting || reaction == ENZF_DMG_REACT_6 {
            return;
        }
        self.last_damage_reaction = reaction;
        let elem = self.body_collider.info;
        self.actor.set_drop_flag(&elem, false);
        if reaction == ENZF_DMG_REACT_STUN || reaction == ENZF_DMG_REACT_ICE {
            if self.action != ENZF_ACTION_STUNNED {
                self.actor.set_color_filter(COLORFILTER_COLORFLAG_BLUE, 120, COLORFILTER_BUFFLAG_OPA, 80);
                self.actor.apply_damage();
                self.setup_stunned(play);
            }
        } else {
            audio_play_actor_sfx2(play, NA_SE_EN_RIZA_CRY);
            self.actor.set_color_filter(COLORFILTER_COLORFLAG_RED, 255, COLORFILTER_BUFFLAG_OPA, 8);
            if self.actor.apply_damage() == 0 {
                let mut drop_params = DROP_TABLE_4;
                self.setup_die(play);
                if self.dinolfos() {
                    drop_params = DROP_TABLE_14;
                }
                let pos = self.actor.world_pos;
                crate::en_item00::item_drop_collectible_random(play, Some(&self.actor), pos, drop_params);
                enemy_start_finishing_blow(play, &self.actor);
            } else {
                if inactive_type(play) != ENZF_TYPE_LIZALFOS_SINGLE {
                    let c = &self.actor.col_chk_info;
                    if c.health as i32 + c.damage as i32 >= 4 && c.health < 4 {
                        self.last_damage_reaction = ENZF_DMG_REACT_FLEE;
                    }
                }
                self.setup_damaged(play);
            }
        }
    }

    /// `EnZf_SetupCirclePlayer`: round Link at `speed` (the Dinolfos twice as fast, half as long),
    /// 8 to 17 frames.
    fn setup_circle_player(&mut self, play: &mut PlayState, speed: f32) {
        self.morph_to_loop(|a| &a.walk_sideways, -1.0);
        self.timer = ((play.rand.zero_one() * 10.0) + 8.0) as i32;
        if self.dinolfos() {
            self.actor.speed_xz = 2.0 * speed;
            self.timer /= 2;
        } else {
            self.actor.speed_xz = speed;
        }
        self.action_state = 0;
        self.actor.world_rot.y = self.actor.shape_rot.y.wrapping_add(0x3FFF);
        self.action = ENZF_ACTION_CIRCLE_PLAYER;
        self.func = Func::CirclePlayer;
    }

    /// `EnZf_ReactToProjectile`: an arrow (or the hookshot) flying at it within 600: a jump attack
    /// (Link within 90 or no floor to either side, and floor 135 ahead; or the hookshot), else a
    /// dodge round Link (8 for one coming from ahead or behind, 4 from a side). Whether one came.
    fn react_to_projectile(&mut self, play: &mut PlayState) -> bool {
        let Some(projectile) = crate::actor_get_projectile_actor(play, &self.actor, play.cur_actor, 600.0) else { return false };
        let (ppos, pid) = play.actors.actor(projectile).map(|a| (a.world_pos, a.id)).unwrap_or((self.actor.world_pos, 0));
        let rel_yaw_towards_projectile = vec3f_yaw(self.actor.world_pos, ppos).wrapping_sub(self.actor.shape_rot.y);
        self.actor.world_rot.y = self.actor.shape_rot.y.wrapping_add(0x3FFF);
        let mut no_floor_ahead_flags = 0;
        if self.test_no_floor_ahead(play, -8.0) {
            no_floor_ahead_flags |= 1;
        }
        if self.test_no_floor_ahead(play, 8.0) {
            no_floor_ahead_flags |= 2;
        }
        self.actor.world_rot.y = self.actor.shape_rot.y;
        if ((self.actor.xz_dist_to_player < 90.0 || no_floor_ahead_flags == 3) && !self.test_no_floor_ahead(play, 135.0)) || pid == crate::ACTOR_ARMS_HOOK {
            self.setup_jump_attack(play);
        } else {
            self.actor.world_rot.y = self.actor.shape_rot.y.wrapping_add(0x3FFF);
            let direction_flags = if no_floor_ahead_flags == 0 { (play.gameplay_frames & 1) as i32 } else { no_floor_ahead_flags };
            let a = abs_i(rel_yaw_towards_projectile);
            if a < 0x2000 || a >= 0x6000 {
                self.setup_circle_player(play, if direction_flags & 1 != 0 { 8.0 } else { -8.0 });
            } else if a < 0x5FFF {
                self.setup_circle_player(play, if direction_flags & 1 != 0 { 4.0 } else { -4.0 });
            }
            // @bug (game): at exactly 0x5FFF no action is picked, yet it says one was.
        }
        true
    }

    /// `EnZf_ReactToProjectileSetDodgeSpeed`: the hopping one's dodge: 6 to a side (the way with
    /// floor), or a higher hop with floor to neither (5, or 10 from a side).
    fn react_to_projectile_set_dodge_speed(&mut self, play: &mut PlayState) -> bool {
        let mut dodge_velocity_y: i16 = 0;
        let Some(projectile) = crate::actor_get_projectile_actor(play, &self.actor, play.cur_actor, 600.0) else { return false };
        let ppos = play.actors.actor(projectile).map(|a| a.world_pos).unwrap_or(self.actor.world_pos);
        let rel_yaw_towards_projectile = vec3f_yaw(self.actor.world_pos, ppos).wrapping_sub(self.actor.shape_rot.y);
        self.actor.world_rot.y = self.actor.shape_rot.y.wrapping_add(0x3FFF);
        let mut no_floor_ahead_flags = 0;
        if self.test_no_floor_ahead(play, -70.0) {
            no_floor_ahead_flags |= 1;
        }
        if self.test_no_floor_ahead(play, 70.0) {
            no_floor_ahead_flags |= 2;
        }
        self.actor.speed_xz = 0.0;
        // @bug (game): at exactly 0x5FFF dodgeSpeed is never set (the stack's garbage; 0 here).
        let mut dodge_speed: i16 = 0;
        let a = abs_i(rel_yaw_towards_projectile);
        let side = if a < 0x2000 || a > 0x5FFF {
            Some(5)
        } else if a < 0x5FFF {
            Some(10)
        } else {
            None
        };
        if let Some(up) = side {
            if no_floor_ahead_flags == 0 {
                dodge_speed = if play.gameplay_frames % 2 != 0 { 6 } else { -6 };
            } else {
                match no_floor_ahead_flags {
                    1 => dodge_speed = 6,
                    2 => dodge_speed = -6,
                    3 => {
                        dodge_velocity_y = up;
                        dodge_speed = 0;
                    }
                    _ => {}
                }
            }
        }
        self.dodge_speed = dodge_speed as f32;
        self.dodge_velocity_y = dodge_velocity_y as f32;
        true
    }

    /// `EffectBlure_AddSpace(Effect_GetByIndex(swordBlureEffectIndex))`.
    fn add_blure_space(&self, play: &mut PlayState) {
        if let Some(b) = play.effect_blure_mut(self.sword_blure_effect_index) {
            b.add_space();
        }
    }

    fn run_action(&mut self, play: &mut PlayState) {
        match self.func {
            Func::WaitAppear => self.wait_appear(play),
            Func::Idle => self.idle(play),
            Func::MainWalk => self.main_walk(play),
            Func::JumpForwards => self.jump_forwards(play),
            Func::TurnTowardsPlayer => self.turn_towards_player(play),
            Func::Sidestep => self.sidestep(play),
            Func::Attack => self.attack(play),
            Func::AttackBounced => self.attack_bounced(play),
            Func::JumpBackwards => self.jump_backwards(play),
            Func::Stunned => self.stunned(play),
            Func::PairedJumpAwayStartTurnAround => self.paired_jump_away_start_turn_around(play),
            Func::PairedIdleHop => self.paired_idle_hop(play),
            Func::PairedJumpAway => self.paired_jump_away(play),
            Func::PairedEngageCombat => self.paired_engage_combat(play),
            Func::Damaged => self.damaged(play),
            Func::JumpAttack => self.jump_attack(play),
            Func::CirclePlayer => self.circle_player(play),
            Func::Die => self.die(play),
        }
    }

    /// `EnZf_OverrideLimbDraw`'s turn: the head's root back by `headRot` (the sheathed lists are
    /// the bakes').
    fn override_limb(limb: usize, rot: &mut [i16; 3], head_rot: i16) {
        if limb == ZF_LIMB_HEAD_ROOT {
            rot[1] = rot[1].wrapping_sub(head_rot);
        }
    }

    /// `EnZf_PostLimbDraw`'s frozen body part of a limb (`bodyPartsPos`).
    fn body_part(limb: usize) -> Option<usize> {
        Some(match limb {
            ZF_LIMB_HEAD => 0,
            ZF_LIMB_NECK => 1,
            ZF_LIMB_ARMOR => 2,
            ZF_LIMB_FOREARM => 3,
            ZF_LIMB_LEFT_FOREARM => 4,
            ZF_LIMB_BODY => 5,
            ZF_LIMB_RIGHT_HAND_AND_SWORD_ROOT => 6,
            ZF_LIMB_RIGHT_SHIN => 7,
            ZF_LIMB_LEFT_SHIN_ROOT => 8,
            _ => return None,
        })
    }
}

/// `rs.values`, `rs.switches`.
mod rs {
    pub const ALPHA: usize = 0;
    pub const SHADOW_ALPHA: usize = 1;
    pub const LEFT_FOOT: usize = 2;
    pub const RIGHT_FOOT: usize = 5;
    pub const DINOLFOS: usize = 0;
    pub const SHEATHED: usize = 1;
}

impl ActorImpl for EnZf {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnZf_Update`.
    fn update(&mut self, play: &mut PlayState) {
        self.check_collide(play);
        if self.actor.col_chk_info.damage_reaction != ENZF_DMG_REACT_6 {
            self.no_floor_ahead = false;
            if self.action_state != 1 && self.action != ENZF_ACTION_PAIRED_JUMP_AWAY {
                if self.actor.speed_xz != 0.0 {
                    self.no_floor_ahead = self.test_no_floor_ahead(play, self.actor.speed_xz * 1.5);
                }
                if !self.no_floor_ahead {
                    self.no_floor_ahead = self.test_no_floor_ahead(play, 0.0);
                }
            }
            if !self.no_floor_ahead {
                self.actor.move_forward();
            }
            let flags = UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2 | UPDBGCHECKINFO_FLAG_3 | UPDBGCHECKINFO_FLAG_4;
            self.actor.update_bg_check_info(&play.col, 25.0, 30.0, 60.0, flags);
            if self.actor.bg_check_flags & BGCHECKFLAG_GROUND == 0 {
                self.action_state = 1;
            }
            self.run_action(play);
        }
        if self.actor.col_chk_info.health > 0 {
            if self.action != ENZF_ACTION_ATTACK && self.action != ENZF_ACTION_STUNNED {
                self.update_head_rot();
            }
            let s = *statics(play);
            if s.s_paired_active_flee_flag != 0 && s.s_paired_inactive_type != self.ty() {
                self.paired_setup_jump_away_start_turn_around(play);
                let ty = self.ty();
                let st = statics(play);
                st.s_paired_inactive_type = ty;
                st.s_paired_active_flee_flag = 0;
                // Signal the other Lizalfos to engage combat (`prev`, else `next`).
                let (prev, next) = self.prev_next(play);
                match prev.or(next) {
                    Some(other) => Self::signal_engage(play, other),
                    None => log::debug!("En_Zf: no actor next to it to signal (a NULL access in the C)"),
                }
            }
        }
        if self.action >= ENZF_ACTION_DIE {
            smooth_step_to_s(&mut self.head_rot, 0, 1, 0x7D0, 0);
            // timer2 is 0 while hopping until the other one sets it: our turn to engage.
            if self.action <= ENZF_ACTION_PAIRED_IDLE_HOP && self.timer2 == 1 && self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
                if self.actor.col_chk_info.health > 0 {
                    self.paired_setup_engage_combat(play);
                }
                self.timer2 -= 1;
            }
            if self.timer2 >= 2 {
                self.timer2 -= 1;
            }
        }
        self.actor.focus_pos = self.actor.world_pos;
        self.actor.focus_pos.y += 40.0;
        if self.actor.col_chk_info.health > 0 && self.alpha == 255 {
            self.body_collider.update(&self.actor);
            if self.actor.world_pos.y == self.actor.floor_height && self.action <= ENZF_ACTION_DAMAGED {
                play.collision_check_set_oc(&self.actor, COL_BODY, &mut self.body_collider);
            }
            let waiting = self.paired() && inactive_type(play) == self.ty();
            if !waiting && !(self.actor.color_filter_timer != 0 && self.actor.color_filter_params & 0x4000 != 0) {
                play.collision_check_set_ac(&self.actor, COL_BODY, &mut self.body_collider);
            }
        }
        if self.action == ENZF_ACTION_ATTACK && self.skel.cur_frame >= 14.0 && self.skel.cur_frame <= 20.0 {
            // (AC_HIT is never set: CollisionCheck_SetAC is never called on the sword.)
            if self.sword_collider.base.at_flags & AT_BOUNCED == 0 && self.sword_collider.base.ac_flags & AC_HIT == 0 {
                play.collision_check_set_at(&self.actor, COL_SWORD, &mut self.sword_collider);
            } else {
                self.sword_collider.base.at_flags &= !AT_BOUNCED;
                self.sword_collider.base.ac_flags &= !AC_HIT;
                self.setup_attack_bounced();
            }
        }
    }

    /// `EnZf_Draw`'s changes to the actor: `EnZf_PostLimbDraw` (the sword's quad and the trail's
    /// vertices from the sword's hand, the feet, the frozen body parts), then opaque and frozen, the
    /// ice: its colour held on, a chunk every 4 frames (`EffectSsEnIce_SpawnFlyingVec3f`).
    fn draw_update(&mut self, play: &mut PlayState) {
        let Some(skeleton) = self.skeleton.clone() else { return };
        let model = actor_draw_matrix(&RenderState::of(&self.actor));
        let joints = self.skel.joint_table.clone();
        let head_rot = self.head_rot;
        let mut posts = Vec::new();
        draw_opa_pose(
            &skeleton,
            &joints,
            |limb, _, rot| {
                Self::override_limb(limb, rot, head_rot);
                LimbDraw::Default(Mat4::IDENTITY)
            },
            |limb, m| posts.push((limb, m)),
        );
        for (limb, m) in posts {
            let m = model * m;
            if limb == ZF_LIMB_RIGHT_HAND_AND_SWORD {
                let q1 = m.transform_point3(D_80B4A2BC);
                let q0 = m.transform_point3(D_80B4A2C8);
                let q3 = m.transform_point3(D_80B4A2D4);
                let q2 = m.transform_point3(D_80B4A2E0);
                self.sword_collider.set_vertices(q0, q1, q2, q3);
                let sp54 = m.transform_point3(D_80B4A2A4);
                let sp48 = m.transform_point3(D_80B4A2B0);
                if self.action == ENZF_ACTION_ATTACK {
                    if self.skel.cur_frame < 14.0 {
                        self.add_blure_space(play);
                    } else if self.skel.cur_frame < 20.0
                        && let Some(b) = play.effect_blure_mut(self.sword_blure_effect_index)
                    {
                        b.add_vertex(sp54, sp48);
                    }
                }
            }
            // (Actor_SetFeetPos on the other limbs: the feet shadow's places, the same as these.)
            match limb {
                ZF_LIMB_LEFT_FOOT_CLAWS => self.left_foot_pos = m.transform_point3(D_80B4A298),
                ZF_LIMB_RIGHT_FOOT_CLAWS => self.right_foot_pos = m.transform_point3(D_80B4A298),
                _ => {}
            }
            if self.frozen_timer != 0
                && let Some(i) = Self::body_part(limb)
            {
                self.body_parts_pos[i] = m.transform_point3(D_80B4A2EC);
            }
        }
        if self.alpha == 255 && self.frozen_timer != 0 {
            self.actor.color_filter_timer = self.actor.color_filter_timer.wrapping_add(1);
            self.frozen_timer -= 1;
            if self.frozen_timer & 3 == 0 {
                let i = (self.frozen_timer >> 2) as usize;
                let pos = self.body_parts_pos.get(i).copied().unwrap_or(Vec3::ZERO);
                let r = self.actor.shape_rot;
                let me = play.cur_actor.map(|h| oot_game::effect::SsActor { handle: h, world_pos: self.actor.world_pos, shape_rot: [r.x, r.y, r.z] });
                play.effect_ss_en_ice_spawn_flying_vec3f(me, pos, [150, 150, 150, 250], [235, 245, 255], 1.4);
            }
        }
    }

    fn render_state(&self) -> RenderState {
        let mut st = RenderState::of(&self.actor);
        st.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        st.angles = vec![self.head_rot];
        let (l, r) = (self.left_foot_pos, self.right_foot_pos);
        st.values = vec![self.alpha as f32, self.shadow_alpha as f32, l.x, l.y, l.z, r.x, r.y, r.z];
        st.switches = vec![self.dinolfos() as u32, self.is_sword_sheathed as u32];
        st
    }

    /// `EnZf_Draw`: the skeleton, opaque at alpha 255 (`ACTOR_SETUP_OPA_DL`), else translucent
    /// (`gActorSetupXluDL`), its env alpha; the head turned; the feet shadow's stand-in.
    fn draw(&self, st: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let (Some(joints), Some(skeleton)) = (&st.joints, &self.skeleton) else { return };
        let [head_rot] = st.angles.as_slice() else { return };
        let (v, sw) = (&st.values, &st.switches);
        if v.len() < 8 || sw.len() < 2 {
            return;
        }
        let (dinolfos, sheathed) = (&sw[rs::DINOLFOS], &sw[rs::SHEATHED]);
        let alpha = v[rs::ALPHA].clamp(0.0, 255.0) as u8;
        let bones = skeleton.pose_override(&joints.rot, |limb, _, rot| {
            Self::override_limb(limb, rot, *head_rot);
            Mat4::IDENTITY
        });
        let xlu = alpha < 255;
        let mut sv = SegmentValues::default();
        sv.env[SEG_ALPHA as usize] = Some([0, 0, 0, alpha]);
        let mesh = MeshKey::named(keys::bake(&bake_name(*dinolfos != 0, *sheathed != 0, xlu)));
        let cmd = DrawCmd { mesh, transform: actor_draw_matrix(st), bones, params: DrawParams { segments: Some(sv), ..Default::default() } };
        if xlu {
            out.xlu.push(cmd);
        } else {
            out.opa.push(cmd);
        }
        if v[rs::SHADOW_ALPHA] > 0.0 {
            // ActorShadow_DrawFeet's stand-in: a disc under each foot.
            for i in [rs::LEFT_FOOT, rs::RIGHT_FOOT] {
                let foot = Vec3::new(v[i], v[i + 1], v[i + 2]);
                let (floor, _) = play.col.entity_raycast_down(foot + Vec3::Y * 20.0);
                let m = Mat4::from_translation(Vec3::new(foot.x, floor + 0.3, foot.z)) * Mat4::from_scale(Vec3::new(12.0, 1.0, 12.0));
                out.xlu.push(DrawCmd::new(MeshKey::named(oot_game::play::builtin::SHADOW), m));
            }
        }
    }

    /// `EnZf_Destroy`: the last paired one with no `En_Zf` left in the enemies within 10000 puts
    /// the music back (`func_800F5B58`); the trail freed. The overlay's statics reset with its
    /// last instance (the overlay's reload).
    fn destroy(&mut self, play: &mut PlayState) {
        // (It's out of the arena here: Actor_FindNearby's skip of itself.)
        if self.paired() && actor_find_nearby(play, &self.actor, None, ACTOR_EN_ZF, ACTORCAT_ENEMY, 10000.0).is_none() {
            play.audio.func_800f5b58();
        }
        play.effect_delete(self.sword_blure_effect_index);
        let others = play.actors.all().into_iter().any(|h| play.actors.actor(h).is_some_and(|a| a.id == ACTOR_EN_ZF));
        if !others {
            play.overlay_statics.remove(&ACTOR_EN_ZF);
        }
    }

    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        match id {
            COL_BODY => Some(ColliderMut::Cylinder(&mut self.body_collider)),
            COL_SWORD => Some(ColliderMut::Quad(&mut self.sword_collider)),
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
