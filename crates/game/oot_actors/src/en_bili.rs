//! `En_Bili` (`ovl_En_Bili/z_en_bili.c`): the Biri, the small electric jellyfish of Jabu-Jabu's
//! Belly (GAME-10 milestone 4), and the three a Bari (`En_Vali`) splits into when it dies.
//! `object_bl`'s `gBiriSkel` (4 limbs: the root, the core, the hood, the tentacles), drawn
//! translucent.
//!
//! Master Quest's Jabu-Jabu (`bdan`) places five, all params 0xFFFF (`EN_BILI_TYPE_NORMAL`) and
//! turned (0, 0, 0), the same in layers 0 to 3: room 1's at (146, -708, -1788) and (-134, -72,
//! -1614), room 2's at (-61, -160, -3267) and (-261, -187, -3066), room 3's at (-10, -1142,
//! -3330). The unused cutscene layer (4) has ten more, all 0xFFFF (rooms 1, 2, 5 and 8). The Bari
//! spawns three with params 0 (`EN_BILI_TYPE_SPAWNED_BY_BARI`, `EnVali_SetupDie`), 0x5555 apart.
//!
//! - **Drifting** (`EnBili_Idle`): `gBiriMainAnim` looped, 0.7 forward, no gravity; every fourth
//!   frame of a count of 32 a random turn (`Rand_CenteredFloat(1820)`). Its height
//!   (`EnBili_UpdateMovement`) follows Link's or the floor's, the higher, 40 over (100 fleeing),
//!   1 a frame (`home.pos.y`), bobbing 3 up and down with the count (`sinf(timer × π / 16)`);
//!   against a wall it takes the wall's yaw. Link within 160 across and 45 up or down, it
//!   **comes at him** (`EnBili_ApproachPlayer`: 1.2, turning to him by halves, 0x71C at most)
//!   until he's over 200 away.
//! - **Spawned by a Bari** (`EnBili_SpawnedByBari`): thrown up at 6 (gravity -0.3) and out at 3,
//!   not hurting, 25 frames, then drifting.
//! - **Its shock** (`EnBili_Electrify`): touching Link (its AT, 8 with `HIT_SPECIAL_EFFECT_ELECTRIC`,
//!   which shocks him) or slashed by a sword (its AC's backlash `HIT_BACKLASH_ELECTRIC` shocks
//!   him back): `gBiriElectrifyAnim` looped, still, shaking (velocity.y 1 and -1 by turns), its
//!   core's colours inverted on odd frames (`sBiriCoreInvertTexCombinerDL`), on each frame of
//!   the four corners (`(timer + 2i) % 4`) a bolt (`EffectSsLightning`, 15, 6 frames, 2 bolts),
//!   `NA_SE_EN_BIRI_SPARK`; 10 frames and the animation's start, then it **rises away**
//!   (`EnBili_AscendAway`: `gBiriAscendAwayAnim` once, not hurting, `NA_SE_EN_BIRI_JUMP` at frame 9
//!   and from there up to 100 over Link, by halves, 5 at most) and **flees** (`EnBili_FleePlayer`:
//!   0.9 away from him, 0x38E a frame, 100 over, 96 frames), or, slashed to death, it dies.
//! - **Hit** (`EnBili_CheckCollide`, health 1, `sDamageTable`): a Deku Nut **stuns** it
//!   (`EnBili_Stunned`: blue 80 frames, falling at gravity -1, no backlash, not hurting,
//!   `NA_SE_EN_GOMA_JR_FREEZE`, `NA_SE_EN_DODO_M_GND` on landing; then drifting); a sword shocks
//!   (unless stunned: then it's damaged); fire arrows and magic burn it (damaged, 2 frames); ice
//!   arrows and magic **freeze** it (`EnBili_Frozen`: eight chunks of ice, `EffectSsEnIce`, blue,
//!   300 frames or until it lands, then it dies); the slingshot **knocks it back**
//!   (`EnBili_SlingshotKnockback`: at 5 away from the seed's last position, up or down toward it,
//!   slowing 0.3 a frame, then turning round and drifting); anything else with damage
//!   **damages** it (`EnBili_Damaged`: red 20 frames, still, not hurting nor hittable, then it
//!   dies). Its last health: `NA_SE_EN_BIRI_DEAD`, the finishing blow, lock-on off. On an arrow
//!   it waits (red) until it's off; an arrow's hit keeps it updating everywhere.
//! - **Dying** (`EnBili_Die`): gone from view, a drop from table 5 (`Item_DropCollectibleRandom`),
//!   then 17 frames of two bubbles each (`EffectSsDtBubble`, 40 to 79 big, 25 frames, mostly
//!   profile 2, else 0), `NA_SE_EN_BIRI_BUBLE` at 14, and gone.
//! - Its tentacles' picture (`EnBili_UpdateTentaclesTexIndex`, one of eight) follows its animation's
//!   frame; at frame 9 of a drift, an approach, a flight or a knockback every other time
//!   `NA_SE_EN_BIRI_FLY`.
//!
//! The whole overlay is ported. Its draw (`EnBili_Draw`, `Gfx_SetupDL_25Xlu`): the tentacles'
//! picture on segment 8, the core's combiner on 9 (inverted on odd frames of its shock), and
//! `gBiriSkel` with the hood, core and tentacles squashed and stretched by the action and the
//! animation's frame (`EnBili_GetHoodLimbScale`, `_GetCoreLimbScale`, `_GetTentaclesLimbScale`
//! before each limb's own transform), the tentacles turned to face the camera. One bake for each
//! picture and combiner (`bake_name`). `ACTOR_FLAG_IGNORE_QUAKE`: it's drawn shaken with the view.
//!
//! Params: `params` whole: -1 placed (`EN_BILI_TYPE_NORMAL`), 0 spawned by the Bari
//! (`EN_BILI_TYPE_SPAWNED_BY_BARI`); 1 (`EN_BILI_TYPE_DIE_AFTER_ELECTRIFY`) it sets itself when a
//! sword's shock takes its last health.
//! Flags: none.

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, MeshKey};
use eng_math::{approach_f, approach_s, scaled_step_to_s, step_to_f, vec3f_pitch, vec3f_yaw};
use glam::{Mat4, Vec3};
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_ENEMY, ActorImpl, ActorProfile, audio_play_actor_sfx2, enemy_start_finishing_blow};
use oot_game::actor_schema::{ActorSchema, Field, Kind};
use oot_game::audio::sfx::{NA_SE_EN_DODO_M_GND, NA_SE_EN_GOMA_JR_FREEZE, SFX_FLAG};
use oot_game::collision_check::*;
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::skelanime_std::{ANIMMODE_LOOP, Anim, LimbDraw, SkelAnimeStd, draw_opa_pose};
use oot_game::sys_matrix::MtxF;

/// `ACTOR_EN_BILI` (`actor_table.h`: 0x0034).
pub const ACTOR_EN_BILI: i16 = 0x0034;
/// `OBJECT_BL` (`object_table.h`: 0x0021).
pub const OBJECT: &str = "object_bl";
pub const OBJECT_BL: i16 = 0x0021;
/// `gBiriSkel` (`object_bl.xml`): 4 limbs (`BIRI_LIMB_MAX` 5 with the root's translation).
pub const SKEL: &str = "gBiriSkel";
/// The overlay's two combiner lists (`ovl_En_Bili.xml`).
pub const OVERLAY: &str = "ovl_En_Bili";

/// The animations (`object_bl.xml`).
pub const ANIM_ELECTRIFY: &str = "gBiriElectrifyAnim";
pub const ANIM_ASCEND_AWAY: &str = "gBiriAscendAwayAnim";
pub const ANIM_MAIN: &str = "gBiriMainAnim";

/// `sTentaclesTextures`: the tentacles' eight pictures.
pub const TENTACLES_TEXTURES: [&str; 8] = ["gBiriTentacle0Tex", "gBiriTentacle1Tex", "gBiriTentacle2Tex", "gBiriTentacle3Tex", "gBiriTentacle4Tex", "gBiriTentacle5Tex", "gBiriTentacle6Tex", "gBiriTentacle7Tex"];

/// `En_Bili_Profile`: `ACTORCAT_ENEMY`, attention-enabled, hostile, shaken with the view, arrows
/// stick in it; `OBJECT_BL`.
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_EN_BILI,
    name: "En_Bili",
    category: ACTORCAT_ENEMY,
    flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE | ACTOR_FLAG_IGNORE_QUAKE | ACTOR_FLAG_CAN_ATTACH_TO_ARROW,
    object: OBJECT,
    schema: &SCHEMA,
};

/// `params` (`EnBiliType`).
pub const TYPE: Field = Field::whole(
    "params",
    Kind::Named(&[(-1, "normal (placed)"), (0, "spawned by a Bari"), (1, "dies after its shock (set by itself)")]),
    "the placed Biri (-1), one thrown out of a dying Bari (0); 1 is set when a sword's shock kills it",
);

/// En_Bili's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema { fields: &[TYPE], flags: &[], todo: false, objects: &[], texts: &[] };

/// `EnBiliType`.
pub const EN_BILI_TYPE_NORMAL: i16 = -1;
pub const EN_BILI_TYPE_SPAWNED_BY_BARI: i16 = 0;
pub const EN_BILI_TYPE_DIE_AFTER_ELECTRIFY: i16 = 1;

/// `ACTOR_FLAG_IGNORE_QUAKE`, `ACTOR_FLAG_CAN_ATTACH_TO_ARROW`, `ACTOR_FLAG_ATTACHED_TO_ARROW`
/// (`actor.h`: 1 << 12, 14, 15).
pub const ACTOR_FLAG_IGNORE_QUAKE: u32 = 1 << 12;
pub const ACTOR_FLAG_CAN_ATTACH_TO_ARROW: u32 = 1 << 14;
pub const ACTOR_FLAG_ATTACHED_TO_ARROW: u32 = 1 << 15;

// The sounds (`include/tables/sfx/enemybank_table.h`).
/// `NA_SE_EN_BIRI_FLY` (0x3898).
pub const NA_SE_EN_BIRI_FLY: u16 = 0x3898;
/// `NA_SE_EN_BIRI_JUMP` (0x3899).
pub const NA_SE_EN_BIRI_JUMP: u16 = 0x3899;
/// `NA_SE_EN_BIRI_SPARK` (0x389A).
pub const NA_SE_EN_BIRI_SPARK: u16 = 0x389A;
/// `NA_SE_EN_BIRI_DEAD` (0x389B).
pub const NA_SE_EN_BIRI_DEAD: u16 = 0x389B;
/// `NA_SE_EN_BIRI_BUBLE` (0x389C).
pub const NA_SE_EN_BIRI_BUBLE: u16 = 0x389C;

/// `NAVI_ENEMY_BIRI` (`actor.h`: 0x17).
pub const NAVI_ENEMY_BIRI: u8 = 0x17;
/// `COLLECTIBLE_DROP_RANDOM_PARAMS(COLLECTIBLE_DROP_TABLE_5, false)` (`z_en_item00.h`: the table
/// × 16).
pub const DROP_TABLE_5: i16 = 5 * 16;

/// `EnBiliDamageReaction`.
pub const EN_BILI_DMG_REACT_NONE: u8 = 0x0;
pub const EN_BILI_DMG_REACT_STUN: u8 = 0x1;
pub const EN_BILI_DMG_REACT_FIRE: u8 = 0x2;
pub const EN_BILI_DMG_REACT_ICE: u8 = 0x3;
pub const EN_BILI_DMG_REACT_SLINGSHOT: u8 = 0xE;
pub const EN_BILI_DMG_REACT_SWORD_SLASH: u8 = 0xF;

/// The limbs `EnBili_OverrideLimbDraw` scales (`BiriLimb`, 1-based).
pub const BIRI_LIMB_CORE: usize = 2;
pub const BIRI_LIMB_HOOD: usize = 3;
pub const BIRI_LIMB_TENTACLES: usize = 4;

/// `sCylinderInit`: `COL_MATERIAL_HIT8`; hurting 8 with an electric shock (`ATELEM_SFX_NONE`),
/// hit by Player's `0xFFCFFFFF` with an electric backlash, pushing as type 1; 9 by 28, 20 down.
pub const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_HIT8, at_flags: AT_ON | AT_TYPE_ENEMY, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0xFFCF_FFFF, hit_special_effect: HIT_SPECIAL_EFFECT_ELECTRIC, damage: 0x08 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: HIT_BACKLASH_ELECTRIC, defense: 0x00 },
        at_elem_flags: ATELEM_ON | ATELEM_SFX_NONE,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 9, height: 28, y_shift: -20, pos: [0; 3] },
};

/// `sColChkInfoInit`: health 1, 9 by 28, 20 down, mass 30.
pub const COL_CHK_INFO_INIT: CollisionCheckInfoInit2 = CollisionCheckInfoInit2 { health: 1, cyl_radius: 9, cyl_height: 28, cyl_y_shift: -20, mass: 30 };

/// `sDamageTable`.
pub static S_DAMAGE_TABLE: DamageTable = DamageTable {
    table: [
        dmg_entry(0, EN_BILI_DMG_REACT_STUN),        // Deku nut
        dmg_entry(2, EN_BILI_DMG_REACT_NONE),        // Deku stick
        dmg_entry(0, EN_BILI_DMG_REACT_SLINGSHOT),   // Slingshot
        dmg_entry(2, EN_BILI_DMG_REACT_NONE),        // Explosive
        dmg_entry(1, EN_BILI_DMG_REACT_NONE),        // Boomerang
        dmg_entry(2, EN_BILI_DMG_REACT_NONE),        // Normal arrow
        dmg_entry(2, EN_BILI_DMG_REACT_NONE),        // Hammer swing
        dmg_entry(2, EN_BILI_DMG_REACT_NONE),        // Hookshot
        dmg_entry(1, EN_BILI_DMG_REACT_SWORD_SLASH), // Kokiri sword
        dmg_entry(2, EN_BILI_DMG_REACT_SWORD_SLASH), // Master sword
        dmg_entry(4, EN_BILI_DMG_REACT_SWORD_SLASH), // Giant's Knife
        dmg_entry(4, EN_BILI_DMG_REACT_FIRE),        // Fire arrow
        dmg_entry(4, EN_BILI_DMG_REACT_ICE),         // Ice arrow
        dmg_entry(2, EN_BILI_DMG_REACT_NONE),        // Light arrow
        dmg_entry(2, EN_BILI_DMG_REACT_NONE),        // Unk arrow 1
        dmg_entry(2, EN_BILI_DMG_REACT_NONE),        // Unk arrow 2
        dmg_entry(2, EN_BILI_DMG_REACT_NONE),        // Unk arrow 3
        dmg_entry(4, EN_BILI_DMG_REACT_FIRE),        // Fire magic
        dmg_entry(4, EN_BILI_DMG_REACT_ICE),         // Ice magic
        dmg_entry(0, EN_BILI_DMG_REACT_NONE),        // Light magic
        dmg_entry(0, EN_BILI_DMG_REACT_NONE),        // Shield
        dmg_entry(0, EN_BILI_DMG_REACT_NONE),        // Mirror Ray
        dmg_entry(1, EN_BILI_DMG_REACT_NONE),        // Kokiri spin
        dmg_entry(4, EN_BILI_DMG_REACT_NONE),        // Giant spin
        dmg_entry(2, EN_BILI_DMG_REACT_NONE),        // Master spin
        dmg_entry(2, EN_BILI_DMG_REACT_NONE),        // Kokiri jump
        dmg_entry(8, EN_BILI_DMG_REACT_NONE),        // Giant jump
        dmg_entry(4, EN_BILI_DMG_REACT_NONE),        // Master jump
        dmg_entry(0, EN_BILI_DMG_REACT_NONE),        // Unknown 1
        dmg_entry(0, EN_BILI_DMG_REACT_NONE),        // Unblockable
        dmg_entry(4, EN_BILI_DMG_REACT_NONE),        // Hammer jump
        dmg_entry(0, EN_BILI_DMG_REACT_NONE),        // Unknown 2
    ],
};

/// The segments `EnBili_Draw` binds: the tentacles' picture, the core's combiner.
const SEG_TENTACLES: u8 = 0x08;
const SEG_COMBINER: u8 = 0x09;
/// `sBiriCoreInvertTexCombinerDL` and `sBiriCoreNormalCombinerDL` (`ovl_En_Bili`, 0x0 and 0x10:
/// `ootx dl-dump ovl_En_Bili ..`): a `gsDPSetCombineLERP` and `gsSPEndDisplayList` each.
const INVERT_COMBINER_DL: [(u32, u32); 2] = [(0xFC62_1603, 0x1F5B_FFF8), (0xDF00_0000, 0)];
const NORMAL_COMBINER_DL: [(u32, u32); 2] = [(0xFC12_1603, 0xFF5B_FFF8), (0xDF00_0000, 0)];

/// The bake of `gBiriSkel` with tentacles picture `tex` (0 to 7) and the core's combiner
/// inverted or not.
pub fn bake_name(tex: usize, invert: bool) -> String {
    format!("En_Bili/tentacles{tex}_{}", if invert { "invert" } else { "normal" })
}

/// `EnBili_Draw`'s meshes: `gBiriSkel` for each tentacles picture and each combiner.
pub fn bakes() -> Vec<MeshBake> {
    let mut v = Vec::new();
    for (i, tex) in TENTACLES_TEXTURES.iter().enumerate() {
        for invert in [false, true] {
            let combiner = if invert { INVERT_COMBINER_DL } else { NORMAL_COMBINER_DL };
            v.push(MeshBake {
                name: bake_name(i, invert),
                object: OBJECT.into(),
                segments: vec![(SEG_TENTACLES, BakeSegment::Texture { file: OBJECT.into(), symbol: (*tex).into() }), (SEG_COMBINER, BakeSegment::Commands(combiner.to_vec()))],
                prelude: vec![],
                body: BakeBody::Skeleton { file: OBJECT.into(), symbol: SKEL.into(), limbs: Vec::new() },
            });
        }
    }
    v
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Idle,
    SpawnedByBari,
    Electrify,
    AscendAway,
    ApproachPlayer,
    FleePlayer,
    SlingshotKnockback,
    Damaged,
    Die,
    Stunned,
    Frozen,
}

impl Action {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Action::Idle => "EnBili_Idle",
            Action::SpawnedByBari => "EnBili_SpawnedByBari",
            Action::Electrify => "EnBili_Electrify",
            Action::AscendAway => "EnBili_AscendAway",
            Action::ApproachPlayer => "EnBili_ApproachPlayer",
            Action::FleePlayer => "EnBili_FleePlayer",
            Action::SlingshotKnockback => "EnBili_SlingshotKnockback",
            Action::Damaged => "EnBili_Damaged",
            Action::Die => "EnBili_Die",
            Action::Stunned => "EnBili_Stunned",
            Action::Frozen => "EnBili_Frozen",
        }
    }
}

struct Anims {
    electrify: Anim,
    ascend_away: Anim,
    main: Anim,
}

impl Anims {
    fn load(play: &PlayState) -> Option<Anims> {
        let a = play.assets.clone()?;
        let get = |s: &str| a.animation(OBJECT, s).map_err(|e| log::error!("En_Bili: {e:#}")).ok();
        Some(Anims { electrify: get(ANIM_ELECTRIFY)?, ascend_away: get(ANIM_ASCEND_AWAY)?, main: get(ANIM_MAIN)? })
    }
}

pub struct EnBili {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    skeleton: Option<Arc<Skeleton>>,
    anims: Option<Anims>,
    pub action: Action,
    pub tentacles_tex_index: u8,
    pub play_fly_sfx: bool,
    pub timer: i16,
    pub collider: ColliderCylinder,
    /// `actor.draw` set (`EnBili_Draw`), not `NULL` (`EnBili_Die`).
    pub drawn: bool,
}

/// `Rand_CenteredFloat(f)` added to an `s16` angle (`rot += f`: the sum a float, truncated).
fn add_f(angle: i16, f: f32) -> i16 {
    ((angle as f32) + f) as i32 as i16
}

/// `EnBili_GetHoodLimbScale`: the hood's scale for `action` at the animation's `frame`
/// (`timer` stunned).
pub fn hood_limb_scale(action: Action, frame: f32, timer: i16) -> Vec3 {
    use std::f32::consts::PI;
    let mut scale = Vec3::ONE;
    match action {
        Action::Electrify => scale.y = 1.0 - (((PI * 0.16667) * frame).sin() * 0.26),
        Action::AscendAway => {
            if frame <= 8.0 {
                scale.y = (((PI / 8.0) * frame).cos() * 0.15) + 0.85;
            } else if frame <= 18.0 {
                let f = ((frame - 8.0) * (PI / 10.0)).cos();
                scale.y = 1.0 - (0.3 * f);
                scale.x = (0.2 * f) + 0.8;
            } else {
                let f = ((frame - 18.0) * (PI * 0.0227)).cos();
                scale.y = (0.31 * f) + 1.0;
                scale.x = 1.0 - (0.4 * f);
            }
            scale.z = scale.x;
        }
        Action::Stunned => {
            let f = ((timer as f32) * (PI / 10.0)).sin() * 0.08;
            scale.x -= f;
            scale.y += f;
            scale.z -= f;
        }
        _ => scale.y = (((PI / 8.0) * frame).cos() * 0.13) + 0.87,
    }
    scale
}

/// `EnBili_GetCoreLimbScale`.
pub fn core_limb_scale(action: Action, frame: f32, timer: i16) -> Vec3 {
    use std::f32::consts::PI;
    let mut scale = Vec3::ONE;
    match action {
        Action::Electrify => scale.y = (((PI * 0.16667) * frame).sin() * 0.2) + 1.0,
        Action::AscendAway => {
            if frame <= 8.0 {
                scale.x = 1.125 - (((PI / 8.0) * frame).cos() * 0.125);
            } else if frame <= 18.0 {
                let f = ((frame - 8.0) * (PI / 10.0)).cos();
                scale.x = (0.275 * f) + 0.975;
                scale.y = 1.25 - (0.25 * f);
            } else {
                let f = ((frame - 18.0) * (PI * 0.0227)).cos();
                scale.x = 1.0 - (0.3 * f);
                scale.y = (0.48 * f) + 1.0;
            }
            scale.z = scale.x;
        }
        Action::Stunned => {
            let f = ((timer as f32) * (PI / 10.0)).sin() * 0.08;
            scale.x += f;
            scale.y -= f;
            scale.z += f;
        }
        _ => scale.y = 1.1 - (((PI / 8.0) * frame).cos() * 0.1),
    }
    scale
}

/// `EnBili_GetTentaclesLimbScale`: only rising away.
pub fn tentacles_limb_scale(action: Action, frame: f32) -> Vec3 {
    use std::f32::consts::PI;
    let mut scale = Vec3::ONE;
    if action == Action::AscendAway {
        if frame <= 8.0 {
            let f = ((PI / 8.0) * frame).cos();
            scale.x = 1.125 - (0.125 * f);
            scale.y = (0.3 * f) + 0.7;
        } else if frame <= 18.0 {
            let f = ((frame - 8.0) * (PI / 10.0)).cos();
            scale.x = (0.325 * f) + 0.925;
            scale.y = 0.95 - (0.55 * f);
        } else {
            let f = ((frame - 18.0) * (PI * 0.0227)).cos();
            scale.x = 1.0 - (0.4 * f);
            scale.y = (0.52 * f) + 1.0;
        }
        scale.z = scale.x;
    }
    scale
}

impl EnBili {
    /// `EnBili_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // sInitChain: naviEnemyId NAVI_ENEMY_BIRI, lockOnArrowOffset 2000.
        actor.navi_enemy_id = NAVI_ENEMY_BIRI;
        actor.target_arrow_offset = 2000.0;
        actor.shape_init(0.0, oot_game::shadow::ShadowDraw::Circle, 17.0);
        actor.shape_shadow_alpha = 155;
        let anims = Anims::load(play);
        let skeleton = play.assets.clone().and_then(|a| a.skeleton(OBJECT, SKEL).map_err(|e| log::error!("En_Bili: {e:#}")).ok());
        // SkelAnime_Init(.., &gBiriSkel, &gBiriMainAnim, .., BIRI_LIMB_MAX).
        let skel = SkelAnimeStd::init_flex(skeleton.as_ref().map(|s| s.limbs.len()).unwrap_or(4), anims.as_ref().map(|a| a.main.clone()));
        actor.col_chk_info.set_info2(Some(&S_DAMAGE_TABLE), &COL_CHK_INFO_INIT);
        let mut this = EnBili {
            actor,
            skel,
            skeleton,
            anims,
            action: Action::Idle,
            tentacles_tex_index: 0,
            play_fly_sfx: false,
            timer: 0,
            collider: ColliderCylinder::new(&CYLINDER_INIT),
            drawn: true,
        };
        if TYPE.get(this.actor.params) == EN_BILI_TYPE_NORMAL {
            this.setup_idle();
        } else {
            this.setup_spawned_by_bari();
        }
        Box::new(this)
    }

    fn anim(&self, f: impl Fn(&Anims) -> &Anim) -> Option<Anim> {
        self.anims.as_ref().map(|a| f(a).clone())
    }

    /// `Animation_PlayLoop`.
    fn play_loop(&mut self, f: impl Fn(&Anims) -> &Anim) {
        if let Some(a) = self.anim(f) {
            let last = a.last_frame();
            self.skel.change(a, 1.0, 0.0, last, ANIMMODE_LOOP, 0.0);
        }
    }

    /// `this->skelAnime.animation == &gBiriMainAnim`.
    fn playing_main(&self) -> bool {
        self.skel.is(ANIM_MAIN)
    }

    /// `EnBili_SetupIdle`: 0.7, the electric backlash back, a count of 32, home at its height, no
    /// gravity, hurting and hittable.
    pub fn setup_idle(&mut self) {
        self.actor.speed_xz = 0.7;
        self.collider.info.ac_dmg_info.hit_backlash = HIT_BACKLASH_ELECTRIC;
        self.timer = 32;
        self.actor.home_pos.y = self.actor.world_pos.y;
        self.actor.gravity = 0.0;
        self.actor.velocity.y = 0.0;
        self.collider.base.at_flags |= AT_ON;
        self.collider.base.ac_flags |= AC_ON;
        self.action = Action::Idle;
    }

    /// `EnBili_SetupSpawnedByBari`: thrown up at 6 (gravity -0.3) and out at 3, 25 frames, not
    /// hurting.
    pub fn setup_spawned_by_bari(&mut self) {
        self.play_loop(|a| &a.main);
        self.timer = 25;
        self.actor.velocity.y = 6.0;
        self.actor.gravity = -0.3;
        self.actor.speed_xz = 3.0;
        self.collider.base.at_flags &= !AT_ON;
        self.action = Action::SpawnedByBari;
    }

    /// `EnBili_SetupElectrify`: `gBiriElectrifyAnim` looped, 10 frames, still, -1 up.
    pub fn setup_electrify(&mut self) {
        self.play_loop(|a| &a.electrify);
        self.timer = 10;
        self.action = Action::Electrify;
        self.actor.speed_xz = 0.0;
        self.actor.velocity.y = -1.0;
    }

    /// `EnBili_SetupAscendAway`: `gBiriAscendAwayAnim` once, not hurting, still.
    fn setup_ascend_away(&mut self) {
        if let Some(a) = self.anim(|a| &a.ascend_away) {
            self.skel.play_once(a);
        }
        self.collider.base.at_flags &= !AT_ON;
        self.action = Action::AscendAway;
        self.actor.speed_xz = 0.0;
        self.actor.velocity.y = 0.0;
    }

    /// `EnBili_SetupApproachPlayer`: 1.2.
    fn setup_approach_player(&mut self) {
        self.actor.speed_xz = 1.2;
        self.action = Action::ApproachPlayer;
    }

    /// `EnBili_SetupFleePlayer`: `gBiriMainAnim` looped, 96 frames, 0.9, home at its height,
    /// hurting.
    fn setup_flee_player(&mut self) {
        self.play_loop(|a| &a.main);
        self.timer = 96;
        self.actor.speed_xz = 0.9;
        self.actor.home_pos.y = self.actor.world_pos.y;
        self.collider.base.at_flags |= AT_ON;
        self.action = Action::FleePlayer;
    }

    /// `EnBili_SetupSlingshotKnockback`: `gBiriMainAnim` (unless it's on), at 5 away from where
    /// the seed was last frame (`ac->prevPos`), pitched toward it.
    fn setup_slingshot_knockback(&mut self, play: &PlayState) {
        if !self.playing_main() {
            self.play_loop(|a| &a.main);
        }
        let prev_pos = self.collider.base.ac.and_then(|h| play.actors.actor(h)).map(|a| a.prev_pos).unwrap_or(self.actor.world_pos);
        // Actor_WorldYawTowardPoint, Actor_WorldPitchTowardPoint.
        self.actor.world_rot.y = vec3f_yaw(self.actor.world_pos, prev_pos).wrapping_add(i16::MIN);
        self.actor.world_rot.x = vec3f_pitch(self.actor.world_pos, prev_pos);
        self.action = Action::SlingshotKnockback;
        self.actor.speed_xz = 5.0;
    }

    /// `EnBili_SetupDamaged`: (rising away, `gBiriMainAnim` again) 20 frames, neither hurting nor
    /// hittable, updating everywhere, still, red 20 frames.
    pub fn setup_damaged(&mut self) {
        if self.action == Action::AscendAway {
            self.play_loop(|a| &a.main);
        }
        self.timer = 20;
        self.collider.base.at_flags &= !AT_ON;
        self.collider.base.ac_flags &= !AC_ON;
        self.actor.flags |= ACTOR_FLAG_UPDATE_CULLING_DISABLED;
        self.actor.speed_xz = 0.0;
        self.actor.set_color_filter(COLORFILTER_COLORFLAG_RED, 200, COLORFILTER_BUFFLAG_XLU, 20);
        self.action = Action::Damaged;
    }

    /// `EnBili_SetupDie`: 18 frames, lock-on off, still.
    pub fn setup_die(&mut self) {
        self.timer = 18;
        self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        self.action = Action::Die;
        self.actor.speed_xz = 0.0;
    }

    /// `EnBili_SetupStunned`: 80 frames, no backlash, falling, still, blue 80 frames,
    /// `NA_SE_EN_GOMA_JR_FREEZE`, not hurting.
    fn setup_stunned(&mut self, play: &mut PlayState) {
        self.timer = 80;
        self.collider.info.ac_dmg_info.hit_backlash = HIT_BACKLASH_NONE;
        self.actor.gravity = -1.0;
        self.actor.speed_xz = 0.0;
        self.actor.set_color_filter(COLORFILTER_COLORFLAG_BLUE, 150, COLORFILTER_BUFFLAG_XLU, 80);
        audio_play_actor_sfx2(play, NA_SE_EN_GOMA_JR_FREEZE);
        self.collider.base.at_flags &= !AT_ON;
        self.action = Action::Stunned;
    }

    /// `EnBili_SetupFrozen`: falling (unless on an arrow), eight chunks of ice up its side at the
    /// four corners 7 out, still, blue, neither hurting nor hittable, 300 frames.
    fn setup_frozen(&mut self, play: &mut PlayState) {
        if self.actor.flags & ACTOR_FLAG_ATTACHED_TO_ARROW == 0 {
            self.actor.gravity = -1.0;
        }
        self.actor.velocity.y = 0.0;
        let w = self.actor.world_pos;
        let r = self.actor.shape_rot;
        let mut eff_y = w.y - 15.0;
        for i in 0..8 {
            let eff_x = (if i & 1 != 0 { 7.0 } else { -7.0 }) + w.x;
            eff_y += 2.5;
            let eff_z = (if i & 4 != 0 { 7.0 } else { -7.0 }) + w.z;
            let scale = (play.rand.zero_one() * 0.2) + 0.7;
            let me = play.cur_actor.map(|h| oot_game::effect::SsActor { handle: h, world_pos: w, shape_rot: [r.x, r.y, r.z] });
            play.effect_ss_en_ice_spawn_flying_vec3f(me, Vec3::new(eff_x, eff_y, eff_z), [150, 150, 150, 250], [235, 245, 255], scale);
        }
        self.actor.speed_xz = 0.0;
        self.actor.set_color_filter(COLORFILTER_COLORFLAG_BLUE, 150, COLORFILTER_BUFFLAG_XLU, 10);
        self.collider.base.at_flags &= !AT_ON;
        self.collider.base.ac_flags &= !AC_ON;
        self.timer = 300;
        self.action = Action::Frozen;
    }

    /// `EnBili_UpdateTentaclesTexIndex`: the tentacles' picture by the action and the
    /// animation's frame (an `s16`).
    pub fn update_tentacles_tex_index(&mut self) {
        let anim_cur_frame = self.skel.cur_frame as i32 as i16 as i32;
        let index: i32 = match self.action {
            Action::Electrify => {
                let n = (3 - anim_cur_frame) as i16 as i32;
                (n.abs() + 5) % 8
            }
            Action::AscendAway => {
                let n = anim_cur_frame >> 1;
                if anim_cur_frame < 10 {
                    if n >= 4 { 3 } else { n }
                } else if anim_cur_frame < 19 {
                    let n = 17 - anim_cur_frame;
                    n.max(0) >> 1
                } else if anim_cur_frame < 37 {
                    ((36 - anim_cur_frame) / 3) + 2
                } else {
                    (40 - anim_cur_frame) >> 1
                }
            }
            _ => anim_cur_frame >> 1,
        };
        self.tentacles_tex_index = index as u8;
    }

    /// `EnBili_UpdateMovement`: see the module's docs.
    fn update_movement(&mut self) {
        let player_y = self.actor.world_pos.y + self.actor.y_dist_to_player;
        let target_y_offset = if self.action == Action::FleePlayer { 100.0 } else { 40.0 };
        let floor_y = self.actor.floor_height;
        let target_y = player_y.max(floor_y);
        step_to_f(&mut self.actor.home_pos.y, target_y + target_y_offset, 1.0);
        self.actor.world_pos.y = ((self.timer as f32 * 0.19634955).sin() * 3.0) + self.actor.home_pos.y;
        if self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 {
            self.actor.world_rot.y = self.actor.wall_yaw;
        }
    }

    /// `EnBili_Idle`.
    fn idle(&mut self, play: &mut PlayState) {
        self.skel.update();
        if self.timer != 0 {
            self.timer -= 1;
        }
        if self.timer % 4 == 0 {
            let r = play.rand.centered_float(1820.0);
            self.actor.world_rot.y = add_f(self.actor.world_rot.y, r);
        }
        self.update_movement();
        if self.timer == 0 {
            self.timer = 32;
        }
        if self.actor.xz_dist_to_player < 160.0 && self.actor.y_dist_to_player.abs() < 45.0 {
            self.setup_approach_player();
        }
    }

    /// `EnBili_SpawnedByBari`: 25 frames, then drifting.
    fn spawned_by_bari(&mut self) {
        self.skel.update();
        if self.timer != 0 {
            self.timer -= 1;
        }
        if self.timer == 0 {
            self.setup_idle();
        }
    }

    /// `EnBili_Electrify`: see the module's docs.
    fn electrify(&mut self, play: &mut PlayState) {
        // sEffEnvColor, sEffPrimColor (passed as the prim and env colours, in that order).
        const EFF_ENV_COLOR: [u8; 4] = [255, 255, 255, 255];
        const EFF_PRIM_COLOR: [u8; 4] = [200, 255, 255, 255];
        for i in 0..4i32 {
            if (self.timer as i32 + (i << 1)) % 4 == 0 {
                let eff_yaw = (play.rand.centered_float(12288.0) as i32 as i16).wrapping_add((i * 0x4000) as i16).wrapping_add(0x2000);
                let w = self.actor.world_pos;
                let x = play.rand.centered_float(5.0) + w.x;
                let y = (play.rand.zero_one() * 5.0) + w.y + 2.5;
                let z = play.rand.centered_float(5.0) + w.z;
                let pos = Vec3::new(x, y, z);
                play.with_ss(|s| s.lightning_spawn(pos, EFF_ENV_COLOR, EFF_PRIM_COLOR, 15, eff_yaw, 6, 2));
            }
        }
        self.skel.update();
        self.actor.play_sfx_flagged(NA_SE_EN_BIRI_SPARK - SFX_FLAG);
        if self.timer != 0 {
            self.timer -= 1;
        }
        self.actor.velocity.y *= -1.0;
        if self.timer == 0 && self.skel.on_frame(0.0) {
            if TYPE.get(self.actor.params) == EN_BILI_TYPE_DIE_AFTER_ELECTRIFY {
                self.setup_die();
            } else {
                self.setup_ascend_away();
            }
        }
    }

    /// `EnBili_AscendAway`: `NA_SE_EN_BIRI_JUMP` at frame 9, then up toward 100 over Link; done,
    /// fleeing.
    fn ascend_away(&mut self, play: &mut PlayState) {
        let anim_finished = self.skel.update();
        let anim_cur_frame = self.skel.cur_frame;
        if self.skel.on_frame(9.0) {
            audio_play_actor_sfx2(play, NA_SE_EN_BIRI_JUMP);
        }
        if anim_cur_frame > 9.0 {
            let target = self.actor.world_pos.y + self.actor.y_dist_to_player + 100.0;
            approach_f(&mut self.actor.world_pos.y, target, 0.5, 5.0);
        }
        if anim_finished {
            self.setup_flee_player();
        }
    }

    /// `EnBili_ApproachPlayer`: turning to Link; over 200 away, drifting again.
    fn approach_player(&mut self) {
        self.skel.update();
        approach_s(&mut self.actor.world_rot.y, self.actor.yaw_towards_player, 2, 0x71C);
        if self.timer != 0 {
            self.timer -= 1;
        }
        self.update_movement();
        if self.timer == 0 {
            self.timer = 32;
        }
        if self.actor.xz_dist_to_player > 200.0 {
            self.setup_idle();
        }
    }

    /// `EnBili_FleePlayer`: turning away from Link, 96 frames, then drifting.
    fn flee_player(&mut self) {
        self.skel.update();
        if self.timer != 0 {
            self.timer -= 1;
        }
        scaled_step_to_s(&mut self.actor.world_rot.y, self.actor.yaw_towards_player.wrapping_add(i16::MIN), 0x38E);
        self.update_movement();
        if self.timer == 0 {
            self.setup_idle();
        }
    }

    /// `EnBili_SlingshotKnockback`: slowing 0.3 a frame; stopped, turned round and drifting.
    fn slingshot_knockback(&mut self) {
        self.skel.update();
        if step_to_f(&mut self.actor.speed_xz, 0.0, 0.3) {
            self.actor.world_rot.y = self.actor.world_rot.y.wrapping_add(i16::MIN);
            self.setup_idle();
        }
    }

    /// `EnBili_Damaged`: on an arrow red until it's off; else 20 frames, then dying.
    fn damaged(&mut self) {
        self.skel.update();
        if self.actor.flags & ACTOR_FLAG_ATTACHED_TO_ARROW != 0 {
            self.actor.color_filter_timer = 20;
        } else {
            if self.timer != 0 {
                self.timer -= 1;
            }
            if self.timer == 0 {
                self.setup_die();
            }
        }
    }

    /// `EnBili_Die`: see the module's docs.
    fn die(&mut self, play: &mut PlayState) {
        // sEffVel (its y written each time), sEffAccel.
        let eff_accel = Vec3::ZERO;
        if self.drawn {
            if self.actor.flags & ACTOR_FLAG_ATTACHED_TO_ARROW == 0 {
                self.drawn = false;
                crate::en_item00::item_drop_collectible_random(play, Some(&self.actor), self.actor.world_pos, DROP_TABLE_5);
            } else {
                return;
            }
        }
        if self.timer != 0 {
            self.timer -= 1;
        }
        if self.timer != 0 {
            for _ in 0..2 {
                let w = self.actor.world_pos;
                let x = (play.rand.zero_one() * 10.0) + w.x - 5.0;
                let y = (play.rand.zero_one() * 5.0) + w.y - 2.5;
                let z = (play.rand.zero_one() * 10.0) + w.z - 5.0;
                let eff_pos = Vec3::new(x, y, z);
                let eff_vel = Vec3::new(0.0, play.rand.zero_one() + 1.0, 0.0);
                let eff_scale = play.rand.s16_offset(40, 40);
                let profile = if play.rand.zero_one() < 0.7 { 2 } else { 0 };
                play.with_ss(|s| s.dt_bubble_spawn_color_profile(eff_pos, eff_vel, eff_accel, eff_scale, 25, profile, 1));
            }
        } else {
            self.actor.kill();
        }
        if self.timer == 14 {
            play.sfx_source_play_sfx_at_fixed_world_pos(self.actor.world_pos, 40, NA_SE_EN_BIRI_BUBLE);
        }
    }

    /// `EnBili_Stunned`: 80 frames (`NA_SE_EN_DODO_M_GND` landing), then drifting.
    fn stunned(&mut self, play: &mut PlayState) {
        if self.timer != 0 {
            self.timer -= 1;
        }
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND_TOUCH != 0 {
            audio_play_actor_sfx2(play, NA_SE_EN_DODO_M_GND);
        }
        if self.timer == 0 {
            self.setup_idle();
        }
    }

    /// `EnBili_Frozen`: falling (unless on an arrow); on the ground or with no floor under it,
    /// dying, else blue. (Its 300 frames count down to nothing.)
    fn frozen(&mut self) {
        if self.timer != 0 {
            self.timer -= 1;
        }
        if self.actor.flags & ACTOR_FLAG_ATTACHED_TO_ARROW == 0 {
            self.actor.gravity = -1.0;
        }
        // OOT_VERSION >= NTSC_1_1: BGCHECKFLAG_GROUND (BGCHECKFLAG_GROUND_TOUCH before).
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 || self.actor.floor_height == eng_collision::bgcheck::BGCHECK_Y_MIN {
            self.actor.color_filter_timer = 0;
            self.setup_die();
        } else {
            self.actor.color_filter_timer = 10;
        }
    }

    /// `EnBili_CheckCollide`: see the module's docs.
    fn check_collide(&mut self, play: &mut PlayState) {
        if self.actor.col_chk_info.health != 0 && self.collider.base.ac_flags & AC_HIT != 0 {
            self.collider.base.ac_flags &= !AC_HIT;
            let elem = self.collider.info;
            self.actor.set_drop_flag(&elem, true);
            let reaction = self.actor.col_chk_info.damage_reaction;
            if reaction != EN_BILI_DMG_REACT_NONE || self.actor.col_chk_info.damage != 0 {
                if self.actor.apply_damage() == 0 {
                    audio_play_actor_sfx2(play, NA_SE_EN_BIRI_DEAD);
                    enemy_start_finishing_blow(play, &self.actor);
                    self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
                }
                if reaction == EN_BILI_DMG_REACT_STUN {
                    if self.action != Action::Stunned {
                        self.setup_stunned(play);
                    }
                } else if reaction == EN_BILI_DMG_REACT_SWORD_SLASH {
                    if self.action != Action::Stunned {
                        self.actor.set_color_filter(COLORFILTER_COLORFLAG_RED, 200, COLORFILTER_BUFFLAG_XLU, 10);
                        if self.actor.col_chk_info.health == 0 {
                            // schema: params written whole (EN_BILI_TYPE_DIE_AFTER_ELECTRIFY)
                            self.actor.params = EN_BILI_TYPE_DIE_AFTER_ELECTRIFY;
                        }
                        self.setup_electrify();
                    } else {
                        self.setup_damaged();
                    }
                } else if reaction == EN_BILI_DMG_REACT_FIRE {
                    self.setup_damaged();
                    self.timer = 2;
                } else if reaction == EN_BILI_DMG_REACT_ICE {
                    self.setup_frozen(play);
                } else if reaction == EN_BILI_DMG_REACT_SLINGSHOT {
                    self.setup_slingshot_knockback(play);
                } else {
                    self.setup_damaged();
                }
                if elem.ac_hit_elem.is_some_and(|h| h.at_dmg_info.dmg_flags & DMG_ARROW != 0) {
                    self.actor.flags |= ACTOR_FLAG_UPDATE_CULLING_DISABLED;
                }
            }
        }
    }

    fn run_action(&mut self, play: &mut PlayState) {
        match self.action {
            Action::Idle => self.idle(play),
            Action::SpawnedByBari => self.spawned_by_bari(),
            Action::Electrify => self.electrify(play),
            Action::AscendAway => self.ascend_away(play),
            Action::ApproachPlayer => self.approach_player(),
            Action::FleePlayer => self.flee_player(),
            Action::SlingshotKnockback => self.slingshot_knockback(),
            Action::Damaged => self.damaged(),
            Action::Die => self.die(play),
            Action::Stunned => self.stunned(play),
            Action::Frozen => self.frozen(),
        }
    }

    /// The scales `EnBili_OverrideLimbDraw` applies: the hood's, the core's, the tentacles'.
    pub fn limb_scales(&self) -> [Vec3; 3] {
        let f = self.skel.cur_frame;
        [hood_limb_scale(self.action, f, self.timer), core_limb_scale(self.action, f, self.timer), tentacles_limb_scale(self.action, f)]
    }

    /// `Actor_Draw`'s model matrix, moved by the main camera's quake offset
    /// (`ACTOR_FLAG_IGNORE_QUAKE`).
    fn actor_mtx(rs: &RenderState, flags: u32, quake: Vec3) -> Mat4 {
        let y_offset = rs.y_offset * rs.scale.y;
        let t = if flags & ACTOR_FLAG_IGNORE_QUAKE != 0 { Vec3::new(rs.pos.x + quake.x, rs.pos.y + (y_offset + quake.y), rs.pos.z + quake.z) } else { Vec3::new(rs.pos.x, rs.pos.y + y_offset, rs.pos.z) };
        let mut m = MtxF::set_translate_rotate_yxz(t.x, t.y, t.z, rs.rot);
        m.scale(rs.scale.x, rs.scale.y, rs.scale.z);
        m.to_mat4()
    }
}

/// `render_state`'s layout: switches drawn, the tentacles' picture, the inverted combiner, the
/// flags; values the hood's, the core's and the tentacles' scales (x, y, z each).
mod rs {
    pub const HOOD: usize = 0;
    pub const CORE: usize = 3;
    pub const TENTACLES: usize = 6;
}

impl ActorImpl for EnBili {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnBili_Update`: see the module's docs.
    fn update(&mut self, play: &mut PlayState) {
        if self.collider.base.at_flags & AT_HIT != 0 {
            self.collider.base.at_flags &= !AT_HIT;
            self.setup_electrify();
        }
        self.check_collide(play);
        self.run_action(play);
        if self.action != Action::Die {
            self.update_tentacles_tex_index();
            if self.skel.on_frame(9.0) && matches!(self.action, Action::Idle | Action::FleePlayer | Action::ApproachPlayer | Action::SlingshotKnockback) {
                if self.play_fly_sfx {
                    audio_play_actor_sfx2(play, NA_SE_EN_BIRI_FLY);
                    self.play_fly_sfx = false;
                } else {
                    self.play_fly_sfx = true;
                }
            }
            if self.action == Action::SlingshotKnockback {
                self.actor.move_xyz();
            } else {
                self.actor.move_forward();
            }
            let (radius, height) = (self.collider.dim.radius as f32, self.collider.dim.height as f32);
            self.actor.update_bg_check_info(&play.col, 5.0, radius, height, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_1 | UPDBGCHECKINFO_FLAG_2);
            self.collider.update(&self.actor);
            if self.collider.base.at_flags & AT_ON != 0 {
                play.collision_check_set_at(&self.actor, 0, &mut self.collider);
            }
            if self.collider.base.ac_flags & AC_ON != 0 {
                play.collision_check_set_ac(&self.actor, 0, &mut self.collider);
            }
            play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
            self.actor.set_focus(0.0);
        }
    }

    /// `EnBili_Destroy`: the cylinder (nothing to free).
    fn destroy(&mut self, _play: &mut PlayState) {}

    /// `EnBili_Draw`'s write: `tentaclesTexIndex` clamped to the last picture.
    fn draw_update(&mut self, _play: &mut PlayState) {
        if self.drawn {
            self.tentacles_tex_index = self.tentacles_tex_index.min(TENTACLES_TEXTURES.len() as u8 - 1);
        }
    }

    fn render_state(&self) -> RenderState {
        let mut r = RenderState::of(&self.actor);
        r.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        let invert = self.action == Action::Electrify && self.timer & 1 != 0;
        let tex = self.tentacles_tex_index.min(TENTACLES_TEXTURES.len() as u8 - 1);
        r.switches = vec![self.drawn as u32, tex as u32, invert as u32, self.actor.flags];
        let [h, c, t] = self.limb_scales();
        r.values = vec![h.x, h.y, h.z, c.x, c.y, c.z, t.x, t.y, t.z];
        // The shape's shadow: none with draw NULL (Actor_Draw isn't called).
        if !self.drawn {
            r.shadow = None;
        }
        r
    }

    /// `EnBili_Draw`: `gBiriSkel` translucent (`SkelAnime_Draw` into `POLY_XLU_DISP`), the
    /// tentacles' picture and the core's combiner bound, `EnBili_OverrideLimbDraw` scaling the
    /// hood, the core and the tentacles before their own transforms and turning the tentacles
    /// to face the camera (`Camera_GetCamDirYaw - shape.rot.y + 0x8000`).
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let [drawn, tex, invert, flags] = rs.switches.as_slice() else { return };
        if *drawn == 0 {
            return;
        }
        let (Some(joints), Some(skeleton)) = (&rs.joints, &self.skeleton) else { return };
        let v = &rs.values;
        if v.len() < 9 {
            return;
        }
        let s = |i: usize| Vec3::new(v[i], v[i + 1], v[i + 2]);
        let (hood, core, tentacles) = (s(rs::HOOD), s(rs::CORE), s(rs::TENTACLES));
        let tentacles_yaw = play.cam_dir_yaw().wrapping_sub(rs.rot[1]).wrapping_add(i16::MIN);
        let bones = draw_opa_pose(
            skeleton,
            &joints.rot,
            |limb, _pos, rot| {
                let scale = match limb {
                    BIRI_LIMB_HOOD => hood,
                    BIRI_LIMB_CORE => core,
                    BIRI_LIMB_TENTACLES => {
                        rot[1] = tentacles_yaw;
                        tentacles
                    }
                    _ => Vec3::ONE,
                };
                // Matrix_Scale(scale.x, scale.y, scale.z, MTXMODE_APPLY).
                LimbDraw::Default(Mat4::from_scale(scale))
            },
            |_, _| {},
        );
        let model = Self::actor_mtx(rs, *flags, play.game_camera.quake_offset);
        let bake = bake_name(*tex as usize, *invert != 0);
        out.xlu.push(DrawCmd { mesh: MeshKey::named(keys::bake(&bake)), transform: model, bones, params: Default::default() });
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
