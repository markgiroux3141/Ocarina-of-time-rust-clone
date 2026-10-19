//! `En_Wf` (`ovl_En_Wf/z_en_wf.c`): the Wolfos and the White Wolfos (GAME-08 milestone 1). Child
//! Link meets a gray one by the Sacred Forest Meadow's entrance (`spot05`, params 0x1F00): its
//! death sets switch flag 0x1F, which sinks the meadow's gate (`Bg_Spot05_Soko`). Others wait in
//! the Forest Temple, the Ice Cavern and a grotto; `En_Encount1` spawns them too (0xFF00).
//!
//! - **Buried** (`EnWf_WaitAppear`): flat (`scale.y` 0) 5 under its home, not targetable, until
//!   Link is within 240 across (distance only); then it rises over 5 frames (`scale.y` and its
//!   shadow growing, `NA_SE_EN_WOLFOS_APPEAR`), howls (`gWolfosHowlAnim` at half speed to frame
//!   7) and stands (`EnWf_Idle`). A White Wolfos with a switch flag starts the mini-boss music
//!   (`func_800F5ACC(NA_BGM_MINI_BOSS)`) as it rises, and its destroy puts the old music back.
//! - **Idle** (`EnWf_Idle`): a few frames, then at Link (`EnWf_RunToAttack`, 70%), round him
//!   (`EnWf_CirclePlayer`), or turning to him (`EnWf_TurnTowardsPlayer`). Running at him it stops
//!   50 to 65 away (200 to 215 while Link is locked on to another enemy) and attacks within 90.
//! - **Every frame it picks** (`EnWf_PickAction`): Link swinging his sword within 100
//!   (`func_800354B4`) and the Wolfos facing him, it **blocks** (`EnWf_Block`: a jump slash
//!   always, else every other frame); from wider, it turns to him and **backflips** away
//!   (`EnWf_BackflipBackward`), or forward over him from a wall (`EnWf_BackflipForward`), or
//!   blocks within 80. A bomb within 80 (`Actor_FindNearby(ACTORCAT_EXPLOSIVE)`) sends it round
//!   Link or backflipping. A hookshot or an arrow flying at it within 600
//!   (`Actor_GetProjectileActor`, `EnWf_ReactToProjectile`) makes it block or dodge.
//! - **Its slash** (`EnWf_Attack`): its claws' spheres (`attackAndBlockCollider`, 4 damage; 8 for
//!   a White Wolfos) at frames 9 to 12 and 17 to 19 (`NA_SE_EN_WOLFOS_ATTACK`), up to 7 more
//!   slashes spinning round; bounced off the shield, it recoils (`EnWf_AttackBouncedOff`).
//! - **Hit** (`EnWf_CheckCollide`, from idle on: its body and tail cylinders; the block's spheres
//!   are hard, a sword bounces off them): **a hit on its tail alone, or from behind it (over
//!   0x4A38 off its facing), does four times the damage.** Nuts, the boomerang and the hookshot
//!   stun it (blue, 120); fire burns it (40 frames, ten flames); it flashes red 8 frames and is
//!   knocked back 4 (`EnWf_Damaged`), 8 health. At 0 it **dies** (`EnWf_Die`): howling, blue
//!   flames (`EffectSsDeadDb`) rising, then a drop from table 13, its switch flag set, gone.
//!
//! The whole overlay is ported. Its draw (`EnWf_Draw`) is its skeleton (gray or white) with its
//! eye texture on segment 8 (six bakes: each skeleton with each of the three eye textures), its
//! head and eyes turned (`EnWf_OverrideLimbDraw`); `EnWf_PostLimbDraw` (in `draw_update`) puts its
//! spheres on their limbs, the tail cylinder 1200 along the tail, and the ten body parts' places
//! for the flames. The circle shadow (0, growing to 70 as it appears) is the shape's
//! (`ActorShape_Init`'s, drawn by `Actor_Draw`). From a bomb or a Bomb Flower's within 80 (`Actor_FindNearby`, `ACTORCAT_EXPLOSIVE`) it
//! backflips away (or circles Link, with a wall behind it); `En_Bom_Chu` isn't ported. Not ported: the hookshot (`Actor_GetProjectileActor` sees
//! arrows only).
//!
//! Params: `type` bits 0 to 7 (`params != 0` is a White Wolfos: `gWolfosWhiteSkel`, scale 0.01,
//! 8 damage, the mini-boss music; 0 gray, `gWolfosGraySkel`, 0.0075); init writes the type over
//! the whole params. `switchFlag` bits 8 to 15 (0xFF none).
//! Flags: reads switch flag `switchFlag` at init (set, it's gone) and sets it at its death.

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::{Cylinder16, Sphere16};
use eng_gfx::{DrawCmd, MeshKey};
use eng_math::{cos_s, sin_s, smooth_step_to_f, smooth_step_to_s, vec3f_yaw};
use glam::{Mat4, Vec3};
use oot_game::actor::*;
use oot_game::actor_ctx::{
    ACTORCAT_ENEMY, ACTORCAT_EXPLOSIVE, ActorImpl, ActorProfile, actor_find_nearby, actor_is_locked_on, actor_other_is_locked_on, actor_spawn_floor_dust_ring, actor_test_floor_in_direction,
    audio_play_actor_sfx2, enemy_start_finishing_blow, func_800354b4, func_800355b8, func_800359b8,
};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind};
use oot_game::audio::sfx::NA_SE_EN_GOMA_JR_FREEZE;
use oot_game::collision_check::*;
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::skelanime_std::{ANIMMODE_LOOP, ANIMMODE_LOOP_INTERP, ANIMMODE_ONCE, ANIMMODE_ONCE_INTERP, Anim, LimbDraw, SkelAnimeStd, draw_opa_pose};

use crate::player::Player;

/// `ACTOR_EN_WF` (`actor_table.h`: 0x01AF).
pub const ACTOR_EN_WF: i16 = 0x01AF;
/// `ACTOR_EN_BOM_CHU` (`actor_table.h`: 0x00DA; not ported).
pub const ACTOR_EN_BOM_CHU: i16 = 0x00DA;
pub const OBJECT: &str = "object_wf";
const SKEL_GRAY: &str = "gWolfosGraySkel";
const SKEL_WHITE: &str = "gWolfosWhiteSkel";

/// `En_Wf_Profile`: `ACTORCAT_ENEMY`, attention-enabled, hostile, always updating.
pub const PROFILE: ActorProfile =
    ActorProfile { id: ACTOR_EN_WF, name: "En_Wf", category: ACTORCAT_ENEMY, flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE | ACTOR_FLAG_UPDATE_CULLING_DISABLED, object: OBJECT, schema: &SCHEMA };

/// The type: params bits 0 to 7 (`PARAMS_GET_U(params, 0, 8)`, written over the params at init).
pub const TYPE: Field = Field::params("type", 0, 8, Kind::Named(&[(0, "the Wolfos (gray)"), (1, "the White Wolfos")]), "which Wolfos (any but 0 is a White Wolfos)");
/// `switchFlag`: `PARAMS_GET_U(params, 8, 8)`.
pub const SWITCH_FLAG: Field = Field::params("switchFlag", 8, 8, Kind::SwitchFlag, "set at its death; set at init, it's gone (0xFF none; a White Wolfos with one plays the mini-boss music)");

/// En_Wf's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[TYPE, SWITCH_FLAG],
    flags: &[FlagUse::new(Flag::Switch("switchFlag")).reads().sets().note("at init it's gone; set at its death")],
    todo: false,
    objects: &[],
    texts: &[],
};

// The sounds (`include/tables/sfx/enemybank_table.h`).
/// `NA_SE_EN_WOLFOS_APPEAR` (0x383C).
pub const NA_SE_EN_WOLFOS_APPEAR: u16 = 0x383C;
/// `NA_SE_EN_WOLFOS_CRY` (0x383E).
pub const NA_SE_EN_WOLFOS_CRY: u16 = 0x383E;
/// `NA_SE_EN_WOLFOS_ATTACK` (0x383F).
pub const NA_SE_EN_WOLFOS_ATTACK: u16 = 0x383F;
/// `NA_SE_EN_WOLFOS_DAMAGE` (0x3843).
pub const NA_SE_EN_WOLFOS_DAMAGE: u16 = 0x3843;
/// `NA_SE_EN_WOLFOS_DEAD` (0x384B).
pub const NA_SE_EN_WOLFOS_DEAD: u16 = 0x384B;
/// `NA_SE_EN_WOLFOS_WALK` (0x385A).
pub const NA_SE_EN_WOLFOS_WALK: u16 = 0x385A;
/// `NA_SE_EN_STAL_JUMP` (0x386C).
pub const NA_SE_EN_STAL_JUMP: u16 = 0x386C;

/// `NA_BGM_MINI_BOSS` (`sequence_table.h`: `Sequence_56`, 0x38).
pub const NA_BGM_MINI_BOSS: u16 = 0x38;

/// `NAVI_ENEMY_WOLFOS`, `NAVI_ENEMY_WHITE_WOLFOS` (`actor.h`: 0x4C, 0x57).
pub const NAVI_ENEMY_WOLFOS: u8 = 0x4C;
pub const NAVI_ENEMY_WHITE_WOLFOS: u8 = 0x57;

/// `COLLECTIBLE_DROP_TABLE_13` (`z_en_item00.h`).
const COLLECTIBLE_DROP_TABLE_13: i16 = 13;

/// `PLAYER_MWA_JUMPSLASH_START` (`player.h`: 17).
const PLAYER_MWA_JUMPSLASH_START: usize = 17;

/// `EnWfAction`.
pub const ENWF_ACTION_WAIT_APPEAR: i32 = 0;
pub const ENWF_ACTION_DIE: i32 = 2;
pub const ENWF_ACTION_DAMAGED: i32 = 3;
pub const ENWF_ACTION_BACKFLIP_FORWARD: i32 = 4;
pub const ENWF_ACTION_BACKFLIP_BACKWARD: i32 = 5;
pub const ENWF_ACTION_IDLE: i32 = 6;
pub const ENWF_ACTION_BLOCK: i32 = 7;
pub const ENWF_ACTION_ATTACK: i32 = 8;
pub const ENWF_ACTION_RUN_TO_ATTACK: i32 = 9;
pub const ENWF_ACTION_TURN_TOWARDS_PLAYER: i32 = 10;
pub const ENWF_ACTION_CIRCLE_PLAYER: i32 = 11;
pub const ENWF_ACTION_ATTACK_BOUNCED_OFF: i32 = 12;
pub const ENWF_ACTION_SHORT_CIRCLE_PLAYER: i32 = 14;
pub const ENWF_ACTION_STUNNED: i32 = 15;

/// `EnWfDamageReaction`.
pub const ENWF_DMG_REACT_NONE: u8 = 0x0;
pub const ENWF_DMG_REACT_STUN: u8 = 0x1;
pub const ENWF_DMG_REACT_ICE_MAGIC: u8 = 0x6;
pub const ENWF_DMG_REACT_LIGHT_MAGIC: u8 = 0xD;
pub const ENWF_DMG_REACT_FIRE: u8 = 0xE;
pub const ENWF_DMG_REACT_F: u8 = 0xF;

/// `WolfosLimb` (`object_wf.xml`; the gray skeleton's `WolfosLimbGray` has the same indices).
pub const WOLFOS_LIMB_ROOT: usize = 1;
pub const WOLFOS_LIMB_LEFT_THIGH: usize = 2;
pub const WOLFOS_LIMB_LEFT_SHIN: usize = 3;
pub const WOLFOS_LIMB_LEFT_ANKLE: usize = 4;
pub const WOLFOS_LIMB_LEFT_FOOT: usize = 5;
pub const WOLFOS_LIMB_TAIL: usize = 6;
pub const WOLFOS_LIMB_WAIST: usize = 7;
pub const WOLFOS_LIMB_RIGHT_THIGH: usize = 8;
pub const WOLFOS_LIMB_RIGHT_SHIN: usize = 9;
pub const WOLFOS_LIMB_RIGHT_ANKLE: usize = 10;
pub const WOLFOS_LIMB_RIGHT_FOOT: usize = 11;
pub const WOLFOS_LIMB_TORSO: usize = 12;
pub const WOLFOS_LIMB_RIGHT_ARM: usize = 13;
pub const WOLFOS_LIMB_RIGHT_FOREARM: usize = 14;
pub const WOLFOS_LIMB_RIGHT_CLAWS: usize = 15;
pub const WOLFOS_LIMB_HEAD_ROOT: usize = 16;
pub const WOLFOS_LIMB_HEAD: usize = 17;
pub const WOLFOS_LIMB_EYES: usize = 18;
pub const WOLFOS_LIMB_LEFT_ARM: usize = 19;
pub const WOLFOS_LIMB_LEFT_FOREARM: usize = 20;
pub const WOLFOS_LIMB_LEFT_CLAWS: usize = 21;
pub const WOLFOS_LIMB_MAX: usize = 22;

/// An element of `sAttackAndBlockJntSphElementsInit`: the claws attack (`ELEM_MATERIAL_UNK0`,
/// 0xFFCFFFFF for 4, AT on with the normal sound); the head and torso are hit (`ELEM_MATERIAL_UNK1`,
/// by 0xFFC1FFFF, hookable) and pushed.
const fn element(claws: bool, limb: u8, center: [i16; 3], radius: i16) -> ColliderJntSphElementInit {
    ColliderJntSphElementInit {
        info: if claws {
            ColliderElementInit {
                elem_material: ELEM_MATERIAL_UNK0,
                at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0xFFCF_FFFF, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x04 },
                ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
                at_elem_flags: ATELEM_ON | ATELEM_SFX_NORMAL,
                ac_elem_flags: ACELEM_NONE,
                oc_elem_flags: OCELEM_NONE,
            }
        } else {
            ColliderElementInit {
                elem_material: ELEM_MATERIAL_UNK1,
                at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0 },
                ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFC1_FFFF, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
                at_elem_flags: ATELEM_NONE,
                ac_elem_flags: ACELEM_ON | ACELEM_HOOKABLE,
                oc_elem_flags: OCELEM_ON,
            }
        },
        limb,
        model_sphere: Sphere16 { center, radius },
        scale: 100,
    }
}

/// `sAttackAndBlockJntSphElementsInit`: the right and left claws (15 each), the head (25, 800
/// along it), the torso (30).
const ATTACK_AND_BLOCK_ELEMENTS: [ColliderJntSphElementInit; 4] = [
    element(true, WOLFOS_LIMB_RIGHT_CLAWS as u8, [0, 0, 0], 15),
    element(true, WOLFOS_LIMB_LEFT_CLAWS as u8, [0, 0, 0], 15),
    element(false, WOLFOS_LIMB_HEAD as u8, [800, 0, 0], 25),
    element(false, WOLFOS_LIMB_TORSO as u8, [0, 0, 0], 30),
];

/// `sAttackAndBlockJntSphInit`: metal, an enemy's attack, hit by Player hard (a sword bounces),
/// pushing everything.
const ATTACK_AND_BLOCK_INIT: ColliderInit =
    ColliderInit { col_type: COL_MATERIAL_METAL, at_flags: AT_ON | AT_TYPE_ENEMY, ac_flags: AC_ON | AC_HARD | AC_TYPE_PLAYER, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_JNTSPH };

/// `sBodyCylinderInit` (20 by 50) and `sTailCylinderInit` (15 by 20, 15 down): hit by Player
/// (0xFFCFFFFF), nothing else.
const fn cylinder(radius: i16, height: i16, y_shift: i16) -> ColliderCylinderInit {
    ColliderCylinderInit {
        base: ColliderInit { col_type: COL_MATERIAL_HIT5, at_flags: AT_NONE, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_NONE, oc_flags2: OC2_NONE, shape: COLSHAPE_CYLINDER },
        info: ColliderElementInit {
            elem_material: ELEM_MATERIAL_UNK1,
            at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0 },
            ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
            at_elem_flags: ATELEM_NONE,
            ac_elem_flags: ACELEM_ON,
            oc_elem_flags: OCELEM_NONE,
        },
        dim: Cylinder16 { radius, height, y_shift, pos: [0, 0, 0] },
    }
}
const BODY_CYLINDER_INIT: ColliderCylinderInit = cylinder(20, 50, 0);
const TAIL_CYLINDER_INIT: ColliderCylinderInit = cylinder(15, 20, -15);

/// `sDamageTable`.
pub static S_DAMAGE_TABLE: DamageTable = DamageTable {
    table: [
        dmg_entry(0, ENWF_DMG_REACT_STUN),        // Deku nut
        dmg_entry(2, ENWF_DMG_REACT_NONE),        // Deku stick
        dmg_entry(1, ENWF_DMG_REACT_NONE),        // Slingshot
        dmg_entry(2, ENWF_DMG_REACT_NONE),        // Explosive
        dmg_entry(0, ENWF_DMG_REACT_STUN),        // Boomerang
        dmg_entry(2, ENWF_DMG_REACT_NONE),        // Normal arrow
        dmg_entry(2, ENWF_DMG_REACT_NONE),        // Hammer swing
        dmg_entry(0, ENWF_DMG_REACT_STUN),        // Hookshot
        dmg_entry(1, ENWF_DMG_REACT_NONE),        // Kokiri sword
        dmg_entry(2, ENWF_DMG_REACT_NONE),        // Master sword
        dmg_entry(4, ENWF_DMG_REACT_NONE),        // Giant's Knife
        dmg_entry(4, ENWF_DMG_REACT_FIRE),        // Fire arrow
        dmg_entry(2, ENWF_DMG_REACT_NONE),        // Ice arrow
        dmg_entry(2, ENWF_DMG_REACT_NONE),        // Light arrow
        dmg_entry(2, ENWF_DMG_REACT_NONE),        // Unk arrow 1
        dmg_entry(2, ENWF_DMG_REACT_NONE),        // Unk arrow 2
        dmg_entry(2, ENWF_DMG_REACT_NONE),        // Unk arrow 3
        dmg_entry(4, ENWF_DMG_REACT_FIRE),        // Fire magic
        dmg_entry(0, ENWF_DMG_REACT_ICE_MAGIC),   // Ice magic
        dmg_entry(3, ENWF_DMG_REACT_LIGHT_MAGIC), // Light magic
        dmg_entry(0, ENWF_DMG_REACT_NONE),        // Shield
        dmg_entry(0, ENWF_DMG_REACT_NONE),        // Mirror Ray
        dmg_entry(1, ENWF_DMG_REACT_NONE),        // Kokiri spin
        dmg_entry(4, ENWF_DMG_REACT_NONE),        // Giant spin
        dmg_entry(2, ENWF_DMG_REACT_NONE),        // Master spin
        dmg_entry(2, ENWF_DMG_REACT_NONE),        // Kokiri jump
        dmg_entry(8, ENWF_DMG_REACT_NONE),        // Giant jump
        dmg_entry(4, ENWF_DMG_REACT_NONE),        // Master jump
        dmg_entry(0, ENWF_DMG_REACT_NONE),        // Unknown 1
        dmg_entry(0, ENWF_DMG_REACT_NONE),        // Unblockable
        dmg_entry(4, ENWF_DMG_REACT_NONE),        // Hammer jump
        dmg_entry(0, ENWF_DMG_REACT_NONE),        // Unknown 2
    ],
};

/// The eye textures, by skeleton (`sWolfosGrayEyeTextures`, `sWolfosWhiteEyeTextures`: open,
/// half, closed; the tables' fourth is the half again).
const EYES_GRAY: [&str; 3] = ["gWolfosGrayEyeOpenTex", "gWolfosGrayEyeHalfTex", "gWolfosGrayEyeClosedTex"];
const EYES_WHITE: [&str; 3] = ["gWolfosWhiteEyeOpenTex", "gWolfosWhiteEyeHalfTex", "gWolfosWhiteEyeClosedTex"];
/// `eyeIndex` (0 to 3) to the texture (`sWolfos*EyeTextures[4]`).
const EYE_TEXTURE: [usize; 4] = [0, 1, 2, 1];
/// The eyes' segment (`gSPSegment(POLY_OPA_DISP++, 8, ...)`).
const SEG_EYES: u8 = 0x08;

/// The bake of a skeleton (`white`) with eye texture `eye` (0 open, 1 half, 2 closed).
pub fn bake_name(white: bool, eye: usize) -> String {
    format!("En_Wf/{}_eye{eye}", if white { "white" } else { "gray" })
}

/// `EnWf_Draw`'s meshes: each skeleton after `Gfx_SetupDL_25Opa`, with each eye texture.
pub fn bakes() -> Vec<MeshBake> {
    let mut v = Vec::new();
    for white in [false, true] {
        for (eye, tex) in (if white { EYES_WHITE } else { EYES_GRAY }).iter().enumerate() {
            v.push(MeshBake {
                name: bake_name(white, eye),
                object: OBJECT.into(),
                segments: vec![(SEG_EYES, BakeSegment::Texture { file: OBJECT.into(), symbol: (*tex).into() })],
                prelude: Vec::new(),
                body: BakeBody::Skeleton { file: OBJECT.into(), symbol: if white { SKEL_WHITE } else { SKEL_GRAY }.into(), limbs: Vec::new() },
            });
        }
    }
    v
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Func {
    WaitAppear,
    Idle,
    RunToAttack,
    TurnTowardsPlayer,
    CirclePlayer,
    Attack,
    AttackBouncedOff,
    BackflipBackward,
    Stunned,
    Damaged,
    BackflipForward,
    Block,
    ShortCirclePlayer,
    Die,
}

impl Func {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Func::WaitAppear => "EnWf_WaitAppear",
            Func::Idle => "EnWf_Idle",
            Func::RunToAttack => "EnWf_RunToAttack",
            Func::TurnTowardsPlayer => "EnWf_TurnTowardsPlayer",
            Func::CirclePlayer => "EnWf_CirclePlayer",
            Func::Attack => "EnWf_Attack",
            Func::AttackBouncedOff => "EnWf_AttackBouncedOff",
            Func::BackflipBackward => "EnWf_BackflipBackward",
            Func::Stunned => "EnWf_Stunned",
            Func::Damaged => "EnWf_Damaged",
            Func::BackflipForward => "EnWf_BackflipForward",
            Func::Block => "EnWf_Block",
            Func::ShortCirclePlayer => "EnWf_ShortCirclePlayer",
            Func::Die => "EnWf_Die",
        }
    }
}

struct Anims {
    attack: Anim,
    backflip: Anim,
    block: Anim,
    howl: Anim,
    run: Anim,
    side_step: Anim,
    damaged: Anim,
    idle: Anim,
}

impl Anims {
    fn load(play: &PlayState) -> Option<Anims> {
        let a = play.assets.clone()?;
        let get = |s: &str| a.animation(OBJECT, s).map_err(|e| log::error!("En_Wf: {e:#}")).ok();
        Some(Anims {
            attack: get("gWolfosAttackAnim")?,
            backflip: get("gWolfosBackflipAnim")?,
            block: get("gWolfosBlockAnim")?,
            howl: get("gWolfosHowlAnim")?,
            run: get("gWolfosRunAnim")?,
            side_step: get("gWolfosSideStepAnim")?,
            damaged: get("gWolfosDamagedAnim")?,
            idle: get("gWolfosIdleAnim")?,
        })
    }
}

/// What the Wolfos reads of Player (`GET_PLAYER(play)`).
#[derive(Default, Clone, Copy)]
struct PlayerView {
    rot_y: i16,
    melee_weapon_state: i8,
    melee_weapon_animation: usize,
}

fn player_view(play: &PlayState) -> PlayerView {
    play.player
        .and_then(|h| play.actors.downcast::<Player>(h))
        .map(|p| PlayerView { rot_y: p.actor.shape_rot.y, melee_weapon_state: p.melee_weapon_state, melee_weapon_animation: p.melee_weapon_animation })
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

pub struct EnWf {
    pub actor: Actor,
    /// `bodyPartsPos`: the ten body parts' places (`Vec3s`) for the flames.
    pub body_parts_pos: [[i16; 3]; 10],
    pub skel: SkelAnimeStd,
    skeleton: Option<Arc<Skeleton>>,
    anims: Option<Anims>,
    /// `action` (`EnWfAction`).
    pub action: i32,
    pub func: Func,
    /// `unk_2E0`: never set (always 0).
    pub unk_2e0: i16,
    pub idle_timer: i16,
    pub on_fire_timer: i16,
    pub last_damage_reaction: u8,
    pub timer: i32,
    pub circle_player_extra_speed: f32,
    /// `unk_2F4`: set to 10 at init, never read.
    pub unk_2f4: f32,
    /// `attackState`: 1 its claws out (AT on), 0 not, -1 blocked mid-swing.
    pub attack_state: i16,
    /// `unk_2FA`: set to 0 by the attack, never read.
    pub unk_2fa: i16,
    pub switch_flag: i16,
    pub circle_player_direction: i16,
    /// `unk_300`: in the air at a hit or a backflip (read only by the draw's buried check).
    pub unk_300: bool,
    pub eye_index: u8,
    pub attack_and_block_collider: ColliderJntSph,
    pub body_collider: ColliderCylinder,
    pub tail_collider: ColliderCylinder,
    /// `unk_4BC`, `unk_4C8`: never set (0, 0, 0).
    pub unk_4bc: Vec3,
    pub unk_4c8: Vec3,
    pub head_rot: i16,
}

impl EnWf {
    fn anim(&self, f: impl Fn(&Anims) -> &Anim) -> Option<Anim> {
        self.anims.as_ref().map(|a| f(a).clone())
    }

    /// `Animation_Change(anim, speed, start, end, mode, morph)`; `end` < 0 the last frame.
    fn change(&mut self, f: impl Fn(&Anims) -> &Anim, speed: f32, start: f32, end: Option<f32>, mode: u8, morph: f32) {
        if let Some(a) = self.anim(f) {
            let end = end.unwrap_or_else(|| a.last_frame());
            self.skel.change(a, speed, start, end, mode, morph);
        }
    }

    /// The last frame of an animation (`Animation_GetLastFrame`).
    fn last_frame(&self, f: impl Fn(&Anims) -> &Anim) -> f32 {
        self.anims.as_ref().map(|a| f(a).last_frame()).unwrap_or(0.0)
    }

    /// `EnWf_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // sInitChain: lockOnArrowOffset 2000, gravity -3.
        actor.target_arrow_offset = 2000.0;
        actor.gravity = -3.0;
        actor.col_chk_info.damage_table = Some(&S_DAMAGE_TABLE);
        actor.shape_init(0.0, oot_game::shadow::ShadowDraw::Circle, 0.0);
        actor.focus_pos = actor.world_pos;
        actor.col_chk_info.mass = MASS_HEAVY;
        actor.col_chk_info.health = 8;
        actor.col_chk_info.cyl_radius = 50;
        actor.col_chk_info.cyl_height = 100;
        let switch_flag = SWITCH_FLAG.get(actor.params);
        actor.params = TYPE.get(actor.params);
        let white = TYPE.get(actor.params) != 0;
        let mut attack_and_block_collider = ColliderJntSph::new(&ATTACK_AND_BLOCK_INIT, &ATTACK_AND_BLOCK_ELEMENTS);
        let skel_name = if white { SKEL_WHITE } else { SKEL_GRAY };
        let skeleton = play.assets.clone().and_then(|a| a.skeleton(OBJECT, skel_name).map_err(|e| log::error!("En_Wf: {e:#}")).ok());
        let anims = Anims::load(play);
        // SkelAnime_InitFlex(.., gWolfosIdleAnim, .., WOLFOS_LIMB_MAX).
        let skel = SkelAnimeStd::init_flex(WOLFOS_LIMB_MAX - 1, anims.as_ref().map(|a| a.idle.clone()));
        if !white {
            actor.scale = Vec3::splat(0.0075);
            actor.navi_enemy_id = NAVI_ENEMY_WOLFOS;
        } else {
            actor.scale = Vec3::splat(0.01);
            attack_and_block_collider.elements[0].info.at_dmg_info.damage = 8;
            attack_and_block_collider.elements[1].info.at_dmg_info.damage = 8;
            actor.navi_enemy_id = NAVI_ENEMY_WHITE_WOLFOS;
        }
        let mut this = EnWf {
            actor,
            body_parts_pos: [[0; 3]; 10],
            skel,
            skeleton,
            anims,
            action: ENWF_ACTION_WAIT_APPEAR,
            func: Func::WaitAppear,
            unk_2e0: 0,
            idle_timer: 0,
            on_fire_timer: 0,
            last_damage_reaction: 0,
            timer: 0,
            circle_player_extra_speed: 0.0,
            unk_2f4: 10.0,
            attack_state: 0,
            unk_2fa: 0,
            switch_flag,
            circle_player_direction: 0,
            unk_300: false,
            eye_index: 0,
            attack_and_block_collider,
            body_collider: ColliderCylinder::new(&BODY_CYLINDER_INIT),
            tail_collider: ColliderCylinder::new(&TAIL_CYLINDER_INIT),
            unk_4bc: Vec3::ZERO,
            unk_4c8: Vec3::ZERO,
            head_rot: 0,
        };
        this.setup_wait_appear();
        if this.switch_flag != 0xFF && play.flags.get_switch(this.switch_flag as i32) {
            this.actor.kill();
        }
        Box::new(this)
    }

    fn white(&self) -> bool {
        TYPE.get(self.actor.params) != 0
    }

    /// `Actor_IsFacingPlayer`.
    fn is_facing_player(&self, max_angle: i16) -> bool {
        abs_i(self.actor.yaw_towards_player.wrapping_sub(self.actor.shape_rot.y)) < max_angle as i32
    }

    /// `EnWf_PickAction`: the block or a backflip from Link's sword, a dodge from a bomb; with
    /// `must_pick`, round Link or an attack. Whether it picked.
    fn pick_action(&mut self, play: &mut PlayState, must_pick: bool) -> bool {
        let player = player_view(play);
        let abs_rel_wall_yaw = abs16(self.actor.wall_yaw.wrapping_sub(self.actor.shape_rot.y));
        let abs_rel_yaw_towards_player = abs16(self.actor.yaw_towards_player.wrapping_sub(self.actor.shape_rot.y));

        if func_800354b4(play, &self.actor, 100.0, 0x2710, 0x2EE0, self.actor.shape_rot.y) {
            if player.melee_weapon_animation == PLAYER_MWA_JUMPSLASH_START {
                self.setup_block();
                return true;
            }
            if play.gameplay_frames % 2 != 0 {
                self.setup_block();
                return true;
            }
        }

        if func_800354b4(play, &self.actor, 100.0, 0x5DC0, 0x2AA8, self.actor.shape_rot.y) {
            self.actor.shape_rot.y = self.actor.yaw_towards_player;
            self.actor.world_rot.y = self.actor.yaw_towards_player;
            if self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 && abs_i(abs_rel_wall_yaw) < 0x2EE0 && self.actor.xz_dist_to_player < 120.0 {
                self.setup_backflip_forward(play);
            } else if player.melee_weapon_animation == PLAYER_MWA_JUMPSLASH_START {
                self.setup_block();
            } else if self.actor.xz_dist_to_player < 80.0 && play.gameplay_frames % 2 != 0 {
                self.setup_block();
            } else {
                self.setup_backflip_backward(play);
            }
            return true;
        } else if let Some(explosive) = actor_find_nearby(play, &self.actor, play.cur_actor, -1, ACTORCAT_EXPLOSIVE, 80.0) {
            let (e_id, e_pos, e_rot_y) = play.actors.actor(explosive).map(|e| (e.id, e.world_pos, e.world_rot.y)).unwrap_or_default();
            self.actor.shape_rot.y = self.actor.yaw_towards_player;
            self.actor.world_rot.y = self.actor.yaw_towards_player;
            // (absRelWallYaw < 0x2EE0 compares the s16: -0x8000 passes.)
            if (self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 && abs_rel_wall_yaw < 0x2EE0) || e_id == ACTOR_EN_BOM_CHU {
                if e_id == ACTOR_EN_BOM_CHU && self.actor.world_pos.distance(e_pos) < 80.0 && self.actor.shape_rot.y.wrapping_sub(e_rot_y).wrapping_add(-0x8000i32 as i16) < 0x3E80 {
                    self.setup_backflip_forward(play);
                } else {
                    self.setup_short_circle_player(play);
                }
            } else {
                self.setup_backflip_backward(play);
            }
            return true;
        }

        if must_pick {
            if abs_rel_yaw_towards_player >= 0x1B58 {
                self.setup_short_circle_player(play);
                return true;
            }
            let yaw_diff = player.rot_y.wrapping_sub(self.actor.shape_rot.y);
            if self.actor.xz_dist_to_player <= 80.0 && !actor_other_is_locked_on(play, &self.actor) && (play.gameplay_frames % 8 != 0 || abs_i(yaw_diff) < 0x38E0) {
                self.setup_attack();
            } else {
                self.setup_circle_player(play);
            }
            return true;
        }
        false
    }

    /// `EnWf_SetupWaitAppear`: buried (flat, 5 under its home), the howl's start held, not
    /// targetable, no gravity.
    fn setup_wait_appear(&mut self) {
        self.change(|a| &a.howl, 0.5, 0.0, Some(7.0), ANIMMODE_ONCE_INTERP, 0.0);
        self.actor.world_pos.y = self.actor.home_pos.y - 5.0;
        self.timer = 20;
        self.unk_300 = false;
        self.action = ENWF_ACTION_WAIT_APPEAR;
        self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        self.actor.scale.y = 0.0;
        self.actor.gravity = 0.0;
        self.func = Func::WaitAppear;
    }

    /// `EnWf_WaitAppear`: until Link is within 240 across; then 5 frames rising, then the howl.
    fn wait_appear(&mut self, play: &mut PlayState) {
        if self.timer > 5 {
            self.actor.world_pos.y = self.actor.home_pos.y - 5.0;
            if self.actor.xz_dist_to_player < 240.0 {
                self.timer = 5;
                self.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED;
                if self.white() && self.switch_flag != 0xFF {
                    play.audio.func_800f5acc(NA_BGM_MINI_BOSS);
                }
            }
        } else if self.timer != 0 {
            self.actor.scale.y += self.actor.scale.x * (1.0 / 5.0);
            self.actor.world_pos.y += 0.5;
            smooth_step_to_f(&mut self.actor.shape_shadow_scale, 70.0, 1.0, 70.0 / 5.0, 0.0);
            self.timer -= 1;
            if self.timer == 0 {
                audio_play_actor_sfx2(play, NA_SE_EN_WOLFOS_APPEAR);
            }
        } else if self.skel.update() {
            self.actor.scale.y = self.actor.scale.x;
            self.actor.gravity = -2.0;
            self.setup_idle(play);
        }
    }

    /// `EnWf_SetupIdle`.
    fn setup_idle(&mut self, play: &mut PlayState) {
        // Animation_MorphToLoop(gWolfosIdleAnim, -4).
        self.change(|a| &a.idle, 1.0, 0.0, None, ANIMMODE_LOOP, -4.0);
        self.action = ENWF_ACTION_IDLE;
        self.timer = ((play.rand.zero_one() * 10.0) + 2.0) as i32;
        self.actor.speed_xz = 0.0;
        self.actor.world_rot.y = self.actor.shape_rot.y;
        self.func = Func::Idle;
    }

    /// `EnWf_Idle`.
    fn idle(&mut self, play: &mut PlayState) {
        let player = player_view(play);
        self.skel.update();
        if self.idle_timer != 0 {
            let yaw = self.actor.yaw_towards_player.wrapping_sub(self.actor.shape_rot.y).wrapping_sub(self.head_rot);
            if abs_i(yaw) > 0x2000 {
                self.idle_timer -= 1;
                return;
            }
            self.idle_timer = 0;
        }
        let yaw = abs16(self.actor.yaw_towards_player.wrapping_sub(self.actor.shape_rot.y));
        if self.react_to_projectile(play) {
            return;
        }
        // unk_2E0 is always 0.
        if self.unk_2e0 != 0 {
            self.unk_2e0 -= 1;
            if yaw < 0x1FFE {
                self.unk_2e0 = 0;
            } else {
                return;
            }
        } else if self.pick_action(play, false) {
            return;
        }
        let yaw = abs16(player.rot_y.wrapping_sub(self.actor.shape_rot.y));
        if self.actor.xz_dist_to_player < 80.0 && player.melee_weapon_state != 0 && yaw >= 0x1F40 {
            self.actor.shape_rot.y = self.actor.yaw_towards_player;
            self.actor.world_rot.y = self.actor.yaw_towards_player;
            self.setup_circle_player(play);
        } else {
            self.timer -= 1;
            if self.timer == 0 {
                if self.is_facing_player(0x1555) {
                    if play.rand.zero_one() > 0.3 {
                        self.setup_run_to_attack();
                    } else {
                        self.setup_circle_player(play);
                    }
                } else {
                    self.setup_turn_towards_player();
                }
                if play.gameplay_frames & 0x5F == 0 {
                    audio_play_actor_sfx2(play, NA_SE_EN_WOLFOS_CRY);
                }
            }
        }
    }

    /// `EnWf_SetupRunToAttack`.
    fn setup_run_to_attack(&mut self) {
        self.change(|a| &a.run, 1.0, 0.0, None, ANIMMODE_LOOP_INTERP, -4.0);
        self.action = ENWF_ACTION_RUN_TO_ATTACK;
        self.func = Func::RunToAttack;
    }

    /// The run's footstep: the animation's frame passed 0 this update (`NA_SE_EN_WOLFOS_WALK` and
    /// a ring of dust).
    fn run_step(&self, play: &mut PlayState, prev_frame: i32, before_cur_frame: i32, abs_play_speed: i32) {
        if prev_frame != self.skel.cur_frame as i32 && before_cur_frame <= 0 && abs_play_speed + prev_frame > 0 {
            audio_play_actor_sfx2(play, NA_SE_EN_WOLFOS_WALK);
            let pos = self.actor.world_pos;
            actor_spawn_floor_dust_ring(play, &self.actor, pos, 20.0, 3, 3.0, 50, 50, true);
        }
    }

    /// `EnWf_RunToAttack`.
    fn run_to_attack(&mut self, play: &mut PlayState) {
        let player = player_view(play);
        let mut sp50 = 0.0;
        if self.react_to_projectile(play) {
            return;
        }
        smooth_step_to_s(&mut self.actor.shape_rot.y, self.actor.yaw_towards_player, 1, 0x2EE, 0);
        self.actor.world_rot.y = self.actor.shape_rot.y;
        if actor_other_is_locked_on(play, &self.actor) {
            sp50 = 150.0;
        }
        if self.actor.xz_dist_to_player <= 50.0 + sp50 {
            smooth_step_to_f(&mut self.actor.speed_xz, -8.0, 1.0, 1.5, 0.0);
        } else if self.actor.xz_dist_to_player > 65.0 + sp50 {
            smooth_step_to_f(&mut self.actor.speed_xz, 8.0, 1.0, 1.5, 0.0);
        } else {
            smooth_step_to_f(&mut self.actor.speed_xz, 0.0, 1.0, 6.65, 0.0);
        }
        self.skel.play_speed = self.actor.speed_xz * 0.175;
        let var_v0 = abs16(player.rot_y.wrapping_sub(self.actor.shape_rot.y));
        if self.actor.xz_dist_to_player < 150.0 + sp50 && player.melee_weapon_state != 0 && var_v0 >= 0x1F40 {
            self.actor.shape_rot.y = self.actor.yaw_towards_player;
            self.actor.world_rot.y = self.actor.yaw_towards_player;
            if play.rand.zero_one() > 0.7 {
                self.setup_circle_player(play);
                return;
            }
        }
        let prev_frame = self.skel.cur_frame as i32;
        self.skel.update();
        let abs_play_speed = self.skel.play_speed.abs();
        let before_cur_frame = (self.skel.cur_frame - abs_play_speed) as i32;
        if !self.is_facing_player(0x11C7) {
            if play.rand.zero_one() > 0.5 {
                self.setup_circle_player(play);
            } else {
                self.setup_idle(play);
            }
        } else if self.actor.xz_dist_to_player < 90.0 + sp50 {
            let temp_v1 = player.rot_y.wrapping_sub(self.actor.shape_rot.y);
            if !actor_other_is_locked_on(play, &self.actor) && (play.rand.zero_one() > 0.03 || (self.actor.xz_dist_to_player <= 80.0 && abs_i(temp_v1) < 0x38E0)) {
                self.setup_attack();
            } else if actor_other_is_locked_on(play, &self.actor) && play.rand.zero_one() > 0.5 {
                self.setup_backflip_backward(play);
            } else {
                self.setup_circle_player(play);
            }
        }
        if !self.pick_action(play, false) {
            if play.gameplay_frames & 0x5F == 0 {
                audio_play_actor_sfx2(play, NA_SE_EN_WOLFOS_CRY);
            }
            self.run_step(play, prev_frame, before_cur_frame, abs_play_speed as i32);
        }
    }

    /// `EnWf_SetupTurnTowardsPlayer`.
    fn setup_turn_towards_player(&mut self) {
        // Animation_MorphToLoop(gWolfosSideStepAnim, -4).
        self.change(|a| &a.side_step, 1.0, 0.0, None, ANIMMODE_LOOP, -4.0);
        self.action = ENWF_ACTION_TURN_TOWARDS_PLAYER;
        self.func = Func::TurnTowardsPlayer;
    }

    /// `EnWf_TurnTowardsPlayer`: a quarter of the way plus 2000 a frame, side-stepping.
    fn turn_towards_player(&mut self, play: &mut PlayState) {
        if self.react_to_projectile(play) || self.pick_action(play, false) {
            return;
        }
        let rel = self.actor.yaw_towards_player.wrapping_sub(self.actor.shape_rot.y);
        let yaw_speed = if rel > 0 { ((rel as f32 * 0.25) + 2000.0) as i32 as i16 } else { ((rel as f32 * 0.25) - 2000.0) as i32 as i16 };
        self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(yaw_speed);
        self.actor.world_rot.y = self.actor.shape_rot.y;
        let mut anim_play_speed = yaw_speed as f32 * 0.5;
        if rel > 0 {
            if anim_play_speed > 1.0 {
                anim_play_speed = 1.0;
            }
        } else if anim_play_speed < -1.0 {
            anim_play_speed = -1.0;
        }
        self.skel.play_speed = -anim_play_speed;
        self.skel.update();
        if self.is_facing_player(0x1555) {
            if play.rand.zero_one() > 0.8 {
                self.setup_circle_player(play);
            } else {
                self.setup_run_to_attack();
            }
        }
        if play.gameplay_frames & 0x5F == 0 {
            audio_play_actor_sfx2(play, NA_SE_EN_WOLFOS_CRY);
        }
    }

    /// `EnWf_SetupCirclePlayer`: running round Link one way or the other, 30 to 59 frames.
    fn setup_circle_player(&mut self, play: &mut PlayState) {
        self.change(|a| &a.run, 1.0, 0.0, None, ANIMMODE_LOOP_INTERP, -4.0);
        self.circle_player_direction = if play.rand.zero_one() > 0.5 { 0x3E80 } else { -0x3E80 };
        self.actor.speed_xz = 6.0;
        self.skel.play_speed = self.actor.speed_xz * 0.175;
        self.actor.world_rot.y = self.actor.shape_rot.y;
        self.timer = ((play.rand.zero_one() * 30.0) + 30.0) as i32;
        self.action = ENWF_ACTION_CIRCLE_PLAYER;
        self.circle_player_extra_speed = 0.0;
        self.func = Func::CirclePlayer;
    }

    /// The circling's turn the other way at a wall or an edge (`EnWf_CirclePlayer` and
    /// `EnWf_ShortCirclePlayer` alike).
    fn circle_move(&mut self, play: &mut PlayState) {
        let (speed, yaw) = (self.actor.speed_xz, self.actor.shape_rot.y);
        if self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 || actor_test_floor_in_direction(&mut self.actor, play, speed, yaw) == 0 {
            // A wall in the way: the other way round.
            let yaw_wall_rel_to_target = if self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 {
                self.actor.wall_yaw.wrapping_sub(self.actor.yaw_towards_player).wrapping_sub(self.circle_player_direction)
            } else {
                0
            };
            if abs_i(yaw_wall_rel_to_target) > 0x2EE0 {
                self.circle_player_direction = self.circle_player_direction.wrapping_neg();
            }
        }
    }

    /// The circling's distance kept 60 to 80 from Link (210 to 230 while he's locked on to
    /// another) and its run's animation speed; the frames for the footstep (`run_step`).
    fn circle_speed(&mut self, play: &mut PlayState) -> (i32, i32, i32) {
        let mut extra_radius = 0.0;
        if actor_other_is_locked_on(play, &self.actor) {
            extra_radius = 150.0;
        }
        if self.actor.xz_dist_to_player <= 60.0 + extra_radius {
            smooth_step_to_f(&mut self.circle_player_extra_speed, -4.0, 1.0, 1.5, 0.0);
        } else if self.actor.xz_dist_to_player > 80.0 + extra_radius {
            smooth_step_to_f(&mut self.circle_player_extra_speed, 4.0, 1.0, 1.5, 0.0);
        } else {
            smooth_step_to_f(&mut self.circle_player_extra_speed, 0.0, 1.0, 6.65, 0.0);
        }
        if self.circle_player_extra_speed != 0.0 {
            self.actor.world_pos.x += sin_s(self.actor.shape_rot.y) * self.circle_player_extra_speed;
            self.actor.world_pos.z += cos_s(self.actor.shape_rot.y) * self.circle_player_extra_speed;
        }
        if self.circle_player_extra_speed.abs() < self.actor.speed_xz.abs() {
            self.skel.play_speed = self.actor.speed_xz * 0.175;
        } else {
            self.skel.play_speed = self.circle_player_extra_speed * 0.175;
        }
        self.skel.play_speed = self.skel.play_speed.clamp(-3.0, 3.0);
        let prev_frame = self.skel.cur_frame as i32;
        self.skel.update();
        let before_cur_frame = (self.skel.cur_frame - self.skel.play_speed.abs()) as i32;
        let abs_play_speed = self.skel.play_speed.abs() as i32;
        (prev_frame, before_cur_frame, abs_play_speed)
    }

    /// `EnWf_CirclePlayer`.
    fn circle_player(&mut self, play: &mut PlayState) {
        let player = player_view(play);
        let target = self.actor.yaw_towards_player.wrapping_add(self.circle_player_direction);
        smooth_step_to_s(&mut self.actor.shape_rot.y, target, 1, 0xFA0, 1);
        if self.react_to_projectile(play) || self.pick_action(play, false) {
            return;
        }
        self.actor.world_rot.y = self.actor.shape_rot.y;
        let sp56 = player.rot_y.wrapping_add(self.circle_player_direction).wrapping_add(-0x8000i32 as i16);
        self.circle_move(play);
        let (prev_frame, before_cur_frame, abs_play_speed) = self.circle_speed(play);
        self.run_step(play, prev_frame, before_cur_frame, abs_play_speed);
        if play.gameplay_frames & 0x5F == 0 {
            audio_play_actor_sfx2(play, NA_SE_EN_WOLFOS_CRY);
        }
        if cos_s(sp56.wrapping_sub(self.actor.shape_rot.y)) < -0.85 && !actor_other_is_locked_on(play, &self.actor) && self.actor.xz_dist_to_player <= 80.0 {
            self.setup_attack();
        } else {
            self.timer -= 1;
            if self.timer == 0 {
                if actor_other_is_locked_on(play, &self.actor) && play.rand.zero_one() > 0.5 {
                    self.setup_backflip_backward(play);
                } else {
                    self.setup_idle(play);
                    self.timer = ((play.rand.zero_one() * 3.0) + 1.0) as i32;
                }
            }
        }
    }

    /// `EnWf_SetupAttack`: the slash to frame 20, facing Link, up to 7 more spinning.
    fn setup_attack(&mut self) {
        if let Some(a) = self.anim(|a| &a.attack) {
            self.skel.play_once(a);
        }
        self.attack_and_block_collider.base.at_flags &= !AT_BOUNCED;
        self.actor.shape_rot.y = self.actor.yaw_towards_player;
        self.action = ENWF_ACTION_ATTACK;
        self.unk_2fa = 0;
        self.timer = 7;
        self.skel.end_frame = 20.0;
        self.actor.speed_xz = 0.0;
        self.func = Func::Attack;
    }

    /// The choice at the end of a slash or its recoil (`EnWf_Attack`, `EnWf_AttackBouncedOff`):
    /// idle if it's lost Link or by chance, else round him or the next pick.
    fn after_attack(&mut self, play: &mut PlayState, abs_yaw_diff_with_player: i16, abs_rel_yaw_towards_player: i16, turned_idle_timer: i16, check_frame_15: bool) {
        if !self.is_facing_player(0x1554) && check_frame_15 {
            self.setup_idle(play);
            self.timer = ((play.rand.zero_one() * 5.0) + 5.0) as i32;
            if abs_rel_yaw_towards_player > 0x32C8 {
                self.idle_timer = turned_idle_timer;
            }
        } else if play.rand.zero_one() > 0.7 || self.actor.xz_dist_to_player >= 120.0 {
            self.setup_idle(play);
            self.timer = ((play.rand.zero_one() * 5.0) + 5.0) as i32;
        } else {
            self.actor.world_rot.y = self.actor.yaw_towards_player;
            if play.rand.zero_one() > 0.7 {
                self.setup_short_circle_player(play);
            } else if abs_yaw_diff_with_player <= 0x2710 {
                if abs_rel_yaw_towards_player > 0x3E80 {
                    self.actor.world_rot.y = self.actor.yaw_towards_player;
                    self.setup_circle_player(play);
                } else {
                    self.pick_action(play, true);
                }
            } else {
                self.setup_circle_player(play);
            }
        }
    }

    /// `EnWf_Attack`: its claws out at frames 9 to 12 and 17 to 19; at frame 15 it can stop
    /// (Link not locked on to it, and away or behind it); at the end, while `timer` lasts, it
    /// spins on (faster each time, dust under it), else picks.
    fn attack(&mut self, play: &mut PlayState) {
        let player = player_view(play);
        let anim_cur_frame = self.skel.cur_frame as i32;
        let abs_yaw_diff_with_player = abs16(player.rot_y.wrapping_sub(self.actor.shape_rot.y));
        let abs_rel_yaw_towards_player = abs16(self.actor.yaw_towards_player.wrapping_sub(self.actor.shape_rot.y));
        self.actor.speed_xz = 0.0;
        if (9..=12).contains(&anim_cur_frame) || (17..=19).contains(&anim_cur_frame) {
            if self.attack_state == 0 {
                audio_play_actor_sfx2(play, NA_SE_EN_WOLFOS_ATTACK);
            }
            self.attack_state = 1;
        } else {
            self.attack_state = 0;
        }
        let stop_at_15 =
            anim_cur_frame == 15 && !actor_is_locked_on(play, &self.actor) && (!self.is_facing_player(0x2000) || self.actor.xz_dist_to_player >= 100.0);
        if stop_at_15 || self.skel.update() {
            if anim_cur_frame != 15 && self.timer != 0 {
                self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add((3276.0 * (1.5 + ((self.timer - 4) as f32 * 0.4))) as i32 as i16);
                let pos = self.actor.world_pos;
                actor_spawn_floor_dust_ring(play, &self.actor, pos, 15.0, 1, 2.0, 50, 50, true);
                self.timer -= 1;
            } else {
                self.after_attack(play, abs_yaw_diff_with_player, abs_rel_yaw_towards_player, 7, anim_cur_frame != 15);
            }
        }
    }

    /// `EnWf_SetupAttackBouncedOff`: the slash backwards at half speed, to frame 1 (or 15 from
    /// its second half).
    fn setup_attack_bounced_off(&mut self) {
        let anim_end_frame = if self.skel.cur_frame as i32 >= 16 { 15.0 } else { 1.0 };
        let start = self.skel.cur_frame - 1.0;
        self.change(|a| &a.attack, -0.5, start, Some(anim_end_frame), ANIMMODE_ONCE_INTERP, 0.0);
        self.action = ENWF_ACTION_ATTACK_BOUNCED_OFF;
        self.attack_state = 0;
        self.func = Func::AttackBouncedOff;
    }

    /// `EnWf_AttackBouncedOff`.
    fn attack_bounced_off(&mut self, play: &mut PlayState) {
        let player = player_view(play);
        let abs_yaw_diff_with_player = abs16(player.rot_y.wrapping_sub(self.actor.shape_rot.y));
        let abs_rel_yaw_towards_player = abs16(self.actor.yaw_towards_player.wrapping_sub(self.actor.shape_rot.y));
        if self.skel.update() {
            self.after_attack(play, abs_yaw_diff_with_player, abs_rel_yaw_towards_player, 30, true);
        }
    }

    /// `EnWf_SetupBackflipBackward`: back at 6, facing Link.
    fn setup_backflip_backward(&mut self, play: &mut PlayState) {
        // Animation_MorphToPlayOnce(gWolfosBackflipAnim, -3).
        self.change(|a| &a.backflip, 1.0, 0.0, None, ANIMMODE_ONCE, -3.0);
        self.timer = 0;
        self.unk_300 = true;
        self.action = ENWF_ACTION_BACKFLIP_BACKWARD;
        self.actor.speed_xz = -6.0;
        self.actor.shape_rot.y = self.actor.yaw_towards_player;
        self.actor.world_rot.y = self.actor.yaw_towards_player;
        audio_play_actor_sfx2(play, NA_SE_EN_STAL_JUMP);
        self.func = Func::BackflipBackward;
    }

    /// `EnWf_BackflipBackward`.
    fn backflip_backward(&mut self, play: &mut PlayState) {
        if self.skel.update() {
            if !actor_other_is_locked_on(play, &self.actor) && self.actor.xz_dist_to_player < 170.0 && self.actor.xz_dist_to_player > 140.0 && play.rand.zero_one() < 0.2 {
                self.setup_run_to_attack();
            } else if play.gameplay_frames % 2 != 0 {
                self.setup_short_circle_player(play);
            } else {
                self.setup_idle(play);
            }
        }
        if play.state_frames & 0x5F == 0 {
            audio_play_actor_sfx2(play, NA_SE_EN_WOLFOS_CRY);
        }
    }

    /// `EnWf_SetupStunned`: held at the damaged animation's first frame.
    fn setup_stunned(&mut self, play: &mut PlayState) {
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
            self.actor.speed_xz = 0.0;
        }
        audio_play_actor_sfx2(play, NA_SE_EN_GOMA_JR_FREEZE);
        // Animation_PlayOnceSetSpeed(gWolfosDamagedAnim, 0).
        self.change(|a| &a.damaged, 0.0, 0.0, None, ANIMMODE_ONCE, 0.0);
        self.action = ENWF_ACTION_STUNNED;
        self.func = Func::Stunned;
    }

    /// `EnWf_Stunned`: until its colour's gone on the ground: dead, or a pick.
    fn stunned(&mut self, play: &mut PlayState) {
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND_TOUCH != 0 {
            self.actor.speed_xz = 0.0;
        }
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
            if self.actor.speed_xz < 0.0 {
                self.actor.speed_xz += 0.05;
            }
            self.unk_300 = false;
        }
        if self.actor.color_filter_timer == 0 && self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
            if self.actor.col_chk_info.health == 0 {
                self.setup_die(play);
            } else {
                self.pick_action(play, true);
            }
        }
    }

    /// `EnWf_SetupDamaged`: knocked back 4 on the ground, facing Link.
    fn setup_damaged(&mut self, play: &mut PlayState) {
        // Animation_MorphToPlayOnce(gWolfosDamagedAnim, -4).
        self.change(|a| &a.damaged, 1.0, 0.0, None, ANIMMODE_ONCE, -4.0);
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
            self.unk_300 = false;
            self.actor.speed_xz = -4.0;
        } else {
            self.unk_300 = true;
        }
        self.idle_timer = 0;
        self.actor.world_rot.y = self.actor.yaw_towards_player;
        audio_play_actor_sfx2(play, NA_SE_EN_WOLFOS_DAMAGE);
        self.action = ENWF_ACTION_DAMAGED;
        self.func = Func::Damaged;
    }

    /// `EnWf_Damaged`.
    fn damaged(&mut self, play: &mut PlayState) {
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND_TOUCH != 0 {
            self.actor.speed_xz = 0.0;
        }
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
            if self.actor.speed_xz < 0.0 {
                self.actor.speed_xz += 0.05;
            }
            self.unk_300 = false;
        }
        smooth_step_to_s(&mut self.actor.shape_rot.y, self.actor.yaw_towards_player, 1, 0x1194, 0);
        if !self.pick_action(play, false) && self.skel.update() && self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
            let abs_rel_wall_yaw = abs16(self.actor.wall_yaw.wrapping_sub(self.actor.shape_rot.y));
            if self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 && abs_i(abs_rel_wall_yaw) < 0x2EE0 && self.actor.xz_dist_to_player < 120.0 {
                self.setup_backflip_forward(play);
            } else if !self.react_to_projectile(play) {
                if self.actor.xz_dist_to_player <= 80.0 && !actor_other_is_locked_on(play, &self.actor) && play.gameplay_frames % 8 != 0 {
                    self.setup_attack();
                } else if play.rand.zero_one() > 0.5 {
                    self.setup_idle(play);
                    self.timer = ((play.rand.zero_one() * 5.0) + 5.0) as i32;
                    self.idle_timer = 30;
                } else {
                    self.setup_backflip_backward(play);
                }
            }
        }
    }

    /// `EnWf_SetupBackflipForward`: the backflip backwards, forward at 6.5 and 15 up.
    fn setup_backflip_forward(&mut self, play: &mut PlayState) {
        let last = self.last_frame(|a| &a.backflip);
        self.change(|a| &a.backflip, -1.0, last, Some(0.0), ANIMMODE_ONCE, -3.0);
        self.timer = 0;
        self.unk_300 = false;
        self.action = ENWF_ACTION_BACKFLIP_FORWARD;
        self.actor.speed_xz = 6.5;
        self.actor.velocity.y = 15.0;
        audio_play_actor_sfx2(play, NA_SE_EN_STAL_JUMP);
        self.actor.world_rot.y = self.actor.shape_rot.y;
        self.func = Func::BackflipForward;
    }

    /// `EnWf_BackflipForward`: sparkles while rising; landed, an attack (or idle while Link is
    /// locked on to another).
    fn backflip_forward(&mut self, play: &mut PlayState) {
        smooth_step_to_s(&mut self.actor.shape_rot.y, self.actor.yaw_towards_player, 1, 0xFA0, 1);
        if self.actor.velocity.y >= 5.0 {
            // @bug (game): unk_4C8 and unk_4BC are never set, so the sparkles are at (0, 0, 0).
            func_800355b8(play, self.unk_4c8);
            func_800355b8(play, self.unk_4bc);
        }
        if self.skel.update() && self.actor.bg_check_flags & (BGCHECKFLAG_GROUND | BGCHECKFLAG_GROUND_TOUCH) != 0 {
            self.actor.shape_rot.x = 0;
            self.actor.shape_rot.y = self.actor.yaw_towards_player;
            self.actor.world_rot.y = self.actor.yaw_towards_player;
            self.actor.velocity.y = 0.0;
            self.actor.speed_xz = 0.0;
            self.actor.world_pos.y = self.actor.floor_height;
            if !actor_other_is_locked_on(play, &self.actor) {
                self.setup_attack();
            } else {
                self.setup_idle(play);
            }
        }
    }

    /// `EnWf_SetupBlock`: the block held 10 frames (a swing's claws called off).
    fn setup_block(&mut self) {
        let anim_end_frame = self.last_frame(|a| &a.block);
        if self.attack_state != 0 {
            self.attack_state = -1;
        }
        self.actor.speed_xz = 0.0;
        self.action = ENWF_ACTION_BLOCK;
        self.timer = 0xA;
        self.change(|a| &a.block, 0.0, 0.0, Some(anim_end_frame), ANIMMODE_ONCE_INTERP, -4.0);
        self.func = Func::Block;
    }

    /// Blocking again (a jump slash, or every other frame) or backflipping away from Link's
    /// sword (`EnWf_Block`).
    fn block_again(&mut self, play: &mut PlayState) {
        let player = player_view(play);
        if player.melee_weapon_animation == PLAYER_MWA_JUMPSLASH_START {
            self.setup_block();
        } else if play.gameplay_frames % 2 != 0 {
            self.setup_block();
        } else {
            self.setup_backflip_backward(play);
        }
    }

    /// `EnWf_Block`.
    fn block(&mut self, play: &mut PlayState) {
        let player = player_view(play);
        if self.timer != 0 {
            self.timer -= 1;
        } else {
            self.skel.play_speed = 1.0;
        }
        if self.skel.update() {
            let rel = self.actor.yaw_towards_player.wrapping_sub(self.actor.shape_rot.y);
            if abs_i(rel) <= 0x4000 && self.actor.xz_dist_to_player < 60.0 && self.actor.y_dist_to_player.abs() < 50.0 {
                if func_800354b4(play, &self.actor, 100.0, 0x2710, 0x4000, self.actor.shape_rot.y) {
                    self.block_again(play);
                } else {
                    let yaw_diff = player.rot_y.wrapping_sub(self.actor.shape_rot.y);
                    if !actor_other_is_locked_on(play, &self.actor) && (play.gameplay_frames % 2 != 0 || abs_i(yaw_diff) < 0x38E0) {
                        self.setup_attack();
                    } else {
                        self.setup_circle_player(play);
                    }
                }
            } else {
                self.setup_circle_player(play);
            }
        } else if self.timer == 0 && func_800354b4(play, &self.actor, 100.0, 0x2710, 0x4000, self.actor.shape_rot.y) {
            self.block_again(play);
        }
    }

    /// `EnWf_SetupShortCirclePlayer`: round Link the way his facing turns from it, 5 to 14
    /// frames.
    fn setup_short_circle_player(&mut self, play: &mut PlayState) {
        self.change(|a| &a.run, 1.0, 0.0, None, ANIMMODE_LOOP_INTERP, -4.0);
        let player = player_view(play);
        let angle = player.rot_y.wrapping_add(self.circle_player_direction);
        let s = sin_s(angle.wrapping_sub(self.actor.yaw_towards_player));
        if s > 0.0 {
            self.circle_player_direction = 0x3E80;
        } else if s < 0.0 {
            self.circle_player_direction = -0x3E80;
        } else if play.rand.zero_one() > 0.5 {
            self.circle_player_direction = 0x3E80;
        } else {
            self.circle_player_direction = -0x3E80;
        }
        self.actor.speed_xz = 6.0;
        self.skel.play_speed = self.actor.speed_xz * 0.175;
        self.actor.world_rot.y = self.actor.shape_rot.y;
        self.circle_player_extra_speed = 0.0;
        self.timer = ((play.rand.zero_one() * 10.0) + 5.0) as i32;
        self.action = ENWF_ACTION_SHORT_CIRCLE_PLAYER;
        self.func = Func::ShortCirclePlayer;
    }

    /// `EnWf_ShortCirclePlayer`: at its end, idle with Link facing it, else an attack within
    /// 80 or a run at him.
    fn short_circle_player(&mut self, play: &mut PlayState) {
        let player = player_view(play);
        let target = self.actor.yaw_towards_player.wrapping_add(self.circle_player_direction);
        smooth_step_to_s(&mut self.actor.shape_rot.y, target, 1, 0xBB8, 1);
        self.circle_move(play);
        self.actor.world_rot.y = self.actor.shape_rot.y;
        let (prev_frame, before_cur_frame, abs_play_speed) = self.circle_speed(play);
        if self.pick_action(play, false) {
            return;
        }
        self.timer -= 1;
        if self.timer == 0 {
            // @bug (game): behindPlayerYawAbs is an s16, so ABS(-0x8000) stays -0x8000 and the
            // check below fails (it's exactly -0x8000 with Link locked on to it, facing it).
            let behind_player_yaw_abs = abs16(player.rot_y.wrapping_sub(self.actor.yaw_towards_player));
            if behind_player_yaw_abs >= 0x3A98 {
                self.setup_idle(play);
                self.timer = ((play.rand.zero_one() * 3.0) + 1.0) as i32;
            } else {
                let behind_player_yaw = player.rot_y.wrapping_sub(self.actor.yaw_towards_player);
                self.actor.world_rot.y = self.actor.shape_rot.y;
                if self.actor.xz_dist_to_player <= 80.0 && !actor_other_is_locked_on(play, &self.actor) && (play.gameplay_frames % 4 == 0 || abs_i(behind_player_yaw) < 0x38E0) {
                    self.setup_attack();
                } else {
                    self.setup_run_to_attack();
                }
            }
        }
        self.run_step(play, prev_frame, before_cur_frame, abs_play_speed);
        if play.gameplay_frames & 0x5F == 0 {
            audio_play_actor_sfx2(play, NA_SE_EN_WOLFOS_CRY);
        }
    }

    /// `EnWf_SetupDie`: the howl, knocked back 6, not targetable.
    fn setup_die(&mut self, play: &mut PlayState) {
        // Animation_MorphToPlayOnce(gWolfosHowlAnim, -4).
        self.change(|a| &a.howl, 1.0, 0.0, None, ANIMMODE_ONCE, -4.0);
        self.actor.world_rot.y = self.actor.yaw_towards_player;
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
            self.unk_300 = false;
            self.actor.speed_xz = -6.0;
        } else {
            self.unk_300 = true;
        }
        self.action = ENWF_ACTION_DIE;
        self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        self.timer = self.skel.anim_length as i32;
        audio_play_actor_sfx2(play, NA_SE_EN_WOLFOS_DEAD);
        self.func = Func::Die;
    }

    /// `EnWf_Die`: blue flames rising, more each frame; at the howl's end a drop from table 13,
    /// its switch flag, gone.
    fn die(&mut self, play: &mut PlayState) {
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND_TOUCH != 0 {
            self.actor.speed_xz = 0.0;
        }
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
            smooth_step_to_f(&mut self.actor.speed_xz, 0.0, 1.0, 0.5, 0.0);
            self.unk_300 = false;
        }
        if self.skel.update() {
            let pos = self.actor.world_pos;
            crate::en_item00::item_drop_collectible_random(play, Some(&self.actor), pos, COLLECTIBLE_DROP_TABLE_13 * 16);
            if self.switch_flag != 0xFF {
                play.flags.set_switch(self.switch_flag as i32);
            }
            self.actor.kill();
        } else {
            let vel_accel = Vec3::new(0.0, 0.5, 0.0);
            self.timer -= 1;
            let mut i = (self.skel.anim_length as i32 - self.timer) >> 1;
            while i >= 0 {
                let x = self.actor.world_pos.x + play.rand.centered_float(60.0);
                let z = self.actor.world_pos.z + play.rand.centered_float(60.0);
                let y = self.actor.world_pos.y + 20.0 + play.rand.centered_float(50.0);
                play.with_ss(|s| s.dead_db_spawn(Vec3::new(x, y, z), vel_accel, vel_accel, 100, 0, [255, 255, 255, 255], [0, 0, 255], 1, 9, 1));
                i -= 1;
            }
        }
    }

    /// `EnWf_UpdateHeadRot`: shaking its head while it waits on Link (idle with `idleTimer`), else
    /// turning it to him (0x5DC a frame, to 0x3127); straight while attacking, held while
    /// stunned.
    fn update_head_rot(&mut self) {
        if self.action == ENWF_ACTION_IDLE && self.idle_timer != 0 {
            self.head_rot = (sin_s((self.idle_timer as i32 * 0x1068) as i16) * 8920.0) as i32 as i16;
        } else if self.action != ENWF_ACTION_STUNNED {
            if self.action != ENWF_ACTION_ATTACK {
                let target = self.actor.yaw_towards_player.wrapping_sub(self.actor.shape_rot.y);
                smooth_step_to_s(&mut self.head_rot, target, 1, 0x5DC, 0);
                self.head_rot = self.head_rot.clamp(-0x3127, 0x3127);
            } else {
                self.head_rot = 0;
            }
        }
    }

    /// `EnWf_CheckCollide`: a sword bounced off the block clears every hit; a hit on its body or
    /// tail (from idle on) does four times the damage on the tail alone or from behind it;
    /// stunned, or hurt (red 8, burning for fire), or dead.
    fn check_collide(&mut self, play: &mut PlayState) {
        if self.attack_and_block_collider.base.ac_flags & AC_BOUNCED != 0 {
            self.attack_and_block_collider.base.ac_flags &= !(AC_HIT | AC_BOUNCED);
            self.body_collider.base.ac_flags &= !AC_HIT;
            self.tail_collider.base.ac_flags &= !AC_HIT;
            return;
        }
        let body_hit = self.body_collider.base.ac_flags & AC_HIT != 0;
        let tail_hit = self.tail_collider.base.ac_flags & AC_HIT != 0;
        if !((body_hit || tail_hit) && self.action >= ENWF_ACTION_IDLE) {
            return;
        }
        let rel = self.actor.yaw_towards_player.wrapping_sub(self.actor.shape_rot.y);
        if !(body_hit || !tail_hit) || abs_i(rel) > 0x4A38 {
            self.actor.col_chk_info.damage = self.actor.col_chk_info.damage.wrapping_mul(4);
        }
        self.body_collider.base.ac_flags &= !AC_HIT;
        self.tail_collider.base.ac_flags &= !AC_HIT;
        let reaction = self.actor.col_chk_info.damage_reaction;
        if reaction == ENWF_DMG_REACT_ICE_MAGIC {
            return;
        }
        self.last_damage_reaction = reaction;
        let elem = self.body_collider.info;
        self.actor.set_drop_flag(&elem, true);
        // OOT_VERSION >= PAL_1_0.
        self.attack_state = 0;
        if reaction == ENWF_DMG_REACT_STUN || reaction == ENWF_DMG_REACT_F {
            if self.action != ENWF_ACTION_STUNNED {
                self.actor.set_color_filter(COLORFILTER_COLORFLAG_BLUE, 120, COLORFILTER_BUFFLAG_OPA, 80);
                self.actor.apply_damage();
                self.setup_stunned(play);
            }
        } else {
            self.actor.set_color_filter(COLORFILTER_COLORFLAG_RED, 255, COLORFILTER_BUFFLAG_OPA, 8);
            if self.last_damage_reaction == ENWF_DMG_REACT_FIRE {
                self.on_fire_timer = 40;
            }
            if self.actor.apply_damage() == 0 {
                self.setup_die(play);
                enemy_start_finishing_blow(play, &self.actor);
            } else {
                self.setup_damaged(play);
            }
        }
    }

    /// `EnWf_ReactToProjectile`: an arrow (or hookshot) flying at it within 600: a block facing
    /// it, else a dodge round Link at double speed or a backflip. Whether one came.
    fn react_to_projectile(&mut self, play: &mut PlayState) -> bool {
        let Some(projectile) = crate::actor_get_projectile_actor(play, &self.actor, play.cur_actor, 600.0) else { return false };
        let ppos = play.actors.actor(projectile).map(|a| a.world_pos).unwrap_or(self.actor.world_pos);
        let rel = vec3f_yaw(self.actor.world_pos, ppos).wrapping_sub(self.actor.shape_rot.y);
        self.actor.world_rot.y = self.actor.shape_rot.y;
        let projectile_dist = self.actor.world_pos.distance(ppos);
        // @bug (game): sqrt of a distance that isn't squared (under 160000 it always passes).
        if abs_i(rel) < 0x2EE0 && (projectile_dist as f64).sqrt() < 400.0 {
            self.setup_block();
        } else {
            self.actor.world_rot.y = self.actor.shape_rot.y.wrapping_add(0x3FFF);
            if abs_i(rel) < 0x2000 || abs_i(rel) >= 0x6000 {
                self.setup_short_circle_player(play);
                self.actor.speed_xz *= 2.0;
            } else if abs_i(rel) < 0x5FFF {
                self.setup_backflip_backward(play);
            }
        }
        // @bug (game): at exactly 0x5FFF no action is picked, yet it says one was.
        true
    }

    fn run_action(&mut self, play: &mut PlayState) {
        match self.func {
            Func::WaitAppear => self.wait_appear(play),
            Func::Idle => self.idle(play),
            Func::RunToAttack => self.run_to_attack(play),
            Func::TurnTowardsPlayer => self.turn_towards_player(play),
            Func::CirclePlayer => self.circle_player(play),
            Func::Attack => self.attack(play),
            Func::AttackBouncedOff => self.attack_bounced_off(play),
            Func::BackflipBackward => self.backflip_backward(play),
            Func::Stunned => self.stunned(play),
            Func::Damaged => self.damaged(play),
            Func::BackflipForward => self.backflip_forward(play),
            Func::Block => self.block(play),
            Func::ShortCirclePlayer => self.short_circle_player(play),
            Func::Die => self.die(play),
        }
    }

    /// The draw's condition (`EnWf_Draw`): not buried in the air (`unk_300` is never set
    /// buried, so always).
    fn drawn(&self) -> bool {
        self.action != ENWF_ACTION_WAIT_APPEAR || !self.unk_300
    }

    /// `EnWf_OverrideLimbDraw`: the head and the eyes turned back by `headRot`.
    fn override_limb(limb: usize, rot: &mut [i16; 3], head_rot: i16) {
        if limb == WOLFOS_LIMB_HEAD || limb == WOLFOS_LIMB_EYES {
            rot[1] = rot[1].wrapping_sub(head_rot);
        }
    }
}

impl ActorImpl for EnWf {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnWf_Update`.
    fn update(&mut self, play: &mut PlayState) {
        self.check_collide(play);
        if self.actor.col_chk_info.damage_reaction != ENWF_DMG_REACT_ICE_MAGIC {
            self.actor.move_forward();
            let flags = UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2 | UPDBGCHECKINFO_FLAG_3 | UPDBGCHECKINFO_FLAG_4;
            self.actor.update_bg_check_info(&play.col, 32.0, 30.0, 60.0, flags);
            self.run_action(play);
            self.update_head_rot();
        }
        if self.actor.bg_check_flags & (BGCHECKFLAG_GROUND | BGCHECKFLAG_GROUND_TOUCH) != 0 {
            let mut rot = self.actor.shape_rot;
            func_800359b8(&self.actor, play, self.actor.shape_rot.y, &mut rot);
            self.actor.shape_rot = rot;
        } else {
            smooth_step_to_s(&mut self.actor.shape_rot.x, 0, 1, 0x3E8, 0);
            smooth_step_to_s(&mut self.actor.shape_rot.z, 0, 1, 0x3E8, 0);
        }
        play.collision_check_set_oc(&self.actor, 0, &mut self.attack_and_block_collider);
        if self.action >= ENWF_ACTION_IDLE && (self.actor.color_filter_timer == 0 || self.actor.color_filter_params & 0x4000 == 0) {
            self.body_collider.update(&self.actor);
            play.collision_check_set_ac(&self.actor, 2, &mut self.tail_collider);
            play.collision_check_set_ac(&self.actor, 1, &mut self.body_collider);
        }
        if self.action == ENWF_ACTION_BLOCK {
            play.collision_check_set_ac(&self.actor, 0, &mut self.attack_and_block_collider);
        }
        if self.attack_state > 0 {
            if self.attack_and_block_collider.base.at_flags & AT_BOUNCED == 0 {
                play.collision_check_set_at(&self.actor, 0, &mut self.attack_and_block_collider);
            } else {
                self.setup_attack_bounced_off();
            }
        }
        self.actor.focus_pos = self.actor.world_pos;
        self.actor.focus_pos.y += 25.0;
        if self.eye_index == 0 {
            if play.rand.zero_one() < 0.2 && play.gameplay_frames % 4 == 0 && self.actor.color_filter_timer == 0 {
                self.eye_index += 1;
            }
        } else {
            self.eye_index = (self.eye_index + 1) & 3;
        }
    }

    /// `EnWf_Draw`'s changes to the actor: `EnWf_PostLimbDraw` (the spheres on their limbs, the
    /// tail cylinder 1200 along the tail, the body parts' places while burning or red), then the
    /// burn: the colour kept on, a flame every 4 frames on the next part down
    /// (`EffectSsEnFire_SpawnVec3s`, 75).
    fn draw_update(&mut self, play: &mut PlayState) {
        if !self.drawn() {
            return;
        }
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
        let parts = self.on_fire_timer != 0 || (self.actor.color_filter_timer != 0 && self.actor.color_filter_params & 0x4000 != 0);
        for (limb, m) in posts {
            let m = model * m;
            self.attack_and_block_collider.update_spheres(limb as u8, &m);
            if limb == WOLFOS_LIMB_TAIL {
                let t = m.transform_point3(Vec3::new(1200.0, 0.0, 0.0));
                self.tail_collider.dim.pos = [t.x as i32 as i16, t.y as i32 as i16, t.z as i32 as i16];
            }
            if parts {
                let idx = match limb {
                    WOLFOS_LIMB_EYES => 0,
                    WOLFOS_LIMB_RIGHT_FOREARM => 1,
                    WOLFOS_LIMB_LEFT_FOREARM => 2,
                    WOLFOS_LIMB_TORSO => 3,
                    WOLFOS_LIMB_WAIST => 4,
                    WOLFOS_LIMB_TAIL => 5,
                    WOLFOS_LIMB_RIGHT_SHIN => 6,
                    // @bug (game): 37 is no limb, so bodyPartsPos[7] stays (0, 0, 0) and a
                    // burning Wolfos has a flame there.
                    37 => 7,
                    WOLFOS_LIMB_RIGHT_ANKLE => 8,
                    WOLFOS_LIMB_LEFT_FOOT => 9,
                    _ => continue,
                };
                let p = m.transform_point3(Vec3::ZERO);
                self.body_parts_pos[idx] = [p.x as i32 as i16, p.y as i32 as i16, p.z as i32 as i16];
            }
        }
        if self.on_fire_timer != 0 {
            self.actor.color_filter_timer = self.actor.color_filter_timer.wrapping_add(1);
            self.on_fire_timer -= 1;
            if self.on_fire_timer % 4 == 0 {
                let i = (self.on_fire_timer >> 2) as usize;
                let p = self.body_parts_pos[i];
                let r = self.actor.shape_rot;
                let me = play.cur_actor.map(|h| oot_game::effect::SsActor { handle: h, world_pos: self.actor.world_pos, shape_rot: [r.x, r.y, r.z] });
                // EffectSsEnFire_SpawnVec3s: flags | 0x8000 (the body part's Vec3s).
                play.effect_ss_en_fire_spawn_vec3f(me, Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32), 75, 0, 0x8000u16 as i16, i as i16);
            }
        }
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        rs.angles = vec![self.head_rot];
        rs.switches = vec![self.white() as u32, EYE_TEXTURE[(self.eye_index & 3) as usize] as u32, self.drawn() as u32];
        rs
    }

    /// `EnWf_Draw`: the skeleton with its eye texture, its head turned.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let (Some(joints), Some(skeleton)) = (&rs.joints, &self.skeleton) else { return };
        let ([head_rot], [white, eye, drawn]) = (rs.angles.as_slice(), rs.switches.as_slice()) else { return };
        if *drawn == 0 {
            return;
        }
        let bones = skeleton.pose_override(&joints.rot, |limb, _, rot| {
            Self::override_limb(limb, rot, *head_rot);
            Mat4::IDENTITY
        });
        let mesh = MeshKey::named(keys::bake(&bake_name(*white != 0, *eye as usize)));
        out.opa.push(DrawCmd { mesh, transform: actor_draw_matrix(rs), bones, params: Default::default() });
    }

    /// `EnWf_Destroy`: a White Wolfos with a switch flag puts the music back
    /// (`func_800F5B58`); its spawner (`En_Encount1`) counted down.
    fn destroy(&mut self, play: &mut PlayState) {
        if self.white() && self.switch_flag != 0xFF {
            play.audio.func_800f5b58();
        }
        if let Some(p) = self.actor.parent.and_then(|h| play.actors.downcast_mut::<crate::en_encount1::EnEncount1>(h))
            && !p.actor.killed
            && p.cur_num_spawn > 0
        {
            p.cur_num_spawn -= 1;
        }
    }

    /// `bodyPartsPos` (`EffectSsEnFire`, `Vec3s`) at 0x14C.
    fn effect_fire_pos(&self, i: usize, _vec3s: bool) -> Vec3 {
        self.body_parts_pos.get(i).map(|p| Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32)).unwrap_or(Vec3::ZERO)
    }

    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        match id {
            0 => Some(ColliderMut::JntSph(&mut self.attack_and_block_collider)),
            1 => Some(ColliderMut::Cylinder(&mut self.body_collider)),
            2 => Some(ColliderMut::Cylinder(&mut self.tail_collider)),
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
