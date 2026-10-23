//! `En_Vali` (`ovl_En_Vali/z_en_vali.c`): the Bari, the big electric jellyfish of Jabu-Jabu's
//! Belly (GAME-10 milestone 8), which drops from above, spins with its tentacles out, and dies
//! into three Biri (`En_Bili`). `object_vali`'s `gBariSkel` (28 limbs: the fangs and the two
//! tentacles; the nucleus, the core and the hood drawn on their own), all translucent.
//!
//! Master Quest's Jabu-Jabu (`bdan`) places one in layers 0 to 3, params 0xFFFF, turned (0, 0,
//! 0): room 6's at (-1367, 411, -2490). The unused cutscene layer (4) has five more (rooms 2, 6
//! and 7), all 0xFFFF.
//!
//! - **Waiting** (`EnVali_WaitPlayerNear`): not drawn, not hittable, no lock-on, at its place;
//!   its floor (`floorHeight`) found once at init (`BgCheck_EntityRaycastDown4`: none, it's
//!   gone) and never again (its update moves it by hand: no bg check). Link within 150 across,
//!   it **drops** (`EnVali_Appear`): drawn, lock-on on, falling at 1, then 1.5 times faster each
//!   frame (40 at most) to the floor, `NA_SE_EN_DODO_M_GND`.
//! - **Spinning** (`EnVali_Spin`, `gBariAttackAnim` looped, morphing 3 frames): rising 1.2 a
//!   frame to 40 over the floor (`spinHeight`), bobbing 8 down with the animation (`sinf(frame ×
//!   π / 80)`), turning by `(frame + 4) × 0.4` degrees a frame (the frame folded at 40), its
//!   roll's sound (`NA_SE_EN_BARI_ROLL`) at frames 16, 30, 42 and 55. Its cylinder and its two
//!   tentacles' quads hurt (8, `HIT_SPECIAL_EFFECT_7`: no shock of its own), the quads swept by
//!   each tentacle's body from its root (`EnVali_PostLimbDraw`: -1000 to 3000 along the limb,
//!   with last frame's edge); it updates only near (no `ACTOR_FLAG_UPDATE_CULLING_DISABLED`).
//!   Link over 250 across, it **goes back up** (`EnVali_Bury`: `gBariIdleAnim`, morphing 10, to
//!   its place by halves, 15 at most) and waits again.
//! - **Touching Link** (any of its three ATs): **its shock** (`EnVali_Electrify`, 20 frames):
//!   no lock-on, not hittable, shaking (1 up and down by turns), on each frame of the four
//!   corners (`(timer + 2i) % 4`) a bolt 12 round it across the camera's view
//!   (`EffectSsLightning`: white in pale cyan, 17 big, 6 frames, 2 bolts), `NA_SE_EN_BIRI_SPARK`,
//!   its core's colours inverted on odd frames (`sBariCoreInvertTexCombinerDL`); then spinning
//!   again (`actionFunc` alone: no new animation), or damaged if a sword set it off.
//! - **Hit** (`EnVali_CheckCollide`, health 2, `sDamageTable`): its cylinder (17 by 35, 15
//!   down) is hit by Player's `0xFFCFFFFF` with an **electric backlash**: a sword shocks Link (the
//!   collision check's, Player's `HIT_BACKLASH_ELECTRIC`). A sword (1, 2 or 4) turns it red (150,
//!   30 frames) and sets off its shock (`params` 1), then **damaged**; a Deku Nut or the
//!   boomerang **stuns** it (`EnVali_Stunned`: `gBariAttackAnim` once, morphing 10; blue 80
//!   frames, no backlash, `NA_SE_EN_GOMA_JR_FREEZE`; falling to the floor from 1, 1 faster each
//!   frame, `NA_SE_EN_DODO_M_GND` landing; then the backlash back and spinning); a sword then
//!   damages it at once. Fire **burns** it (`EnVali_Burnt`: red, 2 frames, then dying); ice
//!   **freezes** it (`EnVali_Frozen`: blue 36 frames, a chunk of ice on every other of its first
//!   16 (`EffectSsEnIce`, flying), then falling to the floor from 1, 1 faster each frame, and
//!   dying); a slingshot seed spins it fast 20 frames (`spinTimer`: 0x800 a frame more, the roll
//!   every sixth frame of 16 to 55); anything else with damage **damages** it (`EnVali_Damaged`:
//!   `gBariDamagedAnim` once, morphing 5, red 30 frames, not hittable; then
//!   `EnVali_Recover`'s `gBariRecoverAnim` and spinning, or with no health left dying). Each hit
//!   with damage: `NA_SE_EN_BARI_DAMAGE` (not a nut's or a seed's); its last health:
//!   `NA_SE_EN_BARI_DEAD`, the finishing blow, lock-on off.
//! - **Dying** (`EnVali_SetupDie`): **three Biri** thrown out of it (`En_Bili`, params 0, its
//!   yaw 0x5555 apart), a drop from table 5 (`Item_DropCollectibleRandom`), `NA_SE_EN_BARI_SPLIT`
//!   where it was, gone from view; then 10 to 19 frames of two bubbles each (`EffectSsDtBubble`,
//!   40 to 79 big, 25 frames, mostly profile 2, else 0), and gone.
//! - Its hood and core (`EnVali_GetHoodScale`, `EnVali_GetCoreScale`) squash and stretch with
//!   the action and the animation's frame; its tentacles stretch out to 1.5 spinning
//!   (`EnVali_UpdateTentaclesLengthFactor`, 0.1 a step), and in and out damaged and recovering.
//!   @bug (game): that step runs in each tentacle root's `OverrideLimbDraw`, so twice a drawn
//!   frame, the right tentacle a step ahead of the left: kept.
//!
//! The whole overlay is ported. Its draw (`EnVali_Draw`, `Gfx_SetupDL_25Xlu`): the core's scroll
//! on segment 8 (`Gfx_TexScroll`, 12 a frame) and its combiner on 9; `EnVali_DrawBody` under
//! `Actor_Draw`'s matrix: the core (`gBariCoreDL`) scaled, three nuclei (`gBariNucleusDL`,
//! facing the camera through segment 1's billboard) turned back from its yaw and set apart, the
//! hood (`gBariHoodDL`) scaled; then `gBariSkel` without those three limbs, each tentacle root
//! stretched along x by its length factor. One bake for each combiner of the core, the nucleus,
//! the hood, and the skeleton. `ACTOR_FLAG_IGNORE_QUAKE`: it's drawn shaken with the view.
//!
//! Params: `params` whole: placed 0xFFFF, written 0 at init (any value placed is lost); 1 is set
//! when a sword's hit sets off its shock (`EnVali_Electrify` then damages it), 0 again once it's
//! damaged.
//! Flags: none.

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use eng_math::{cos_s, sin_s, smooth_step_to_f, step_to_f};
use glam::{Mat4, Vec3};
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_ENEMY, ActorImpl, ActorProfile, audio_play_actor_sfx2, enemy_start_finishing_blow};
use oot_game::actor_schema::{ActorSchema, Field, Kind};
use oot_game::audio::sfx::{NA_SE_EN_DODO_M_GND, NA_SE_EN_GOMA_JR_FREEZE, SFX_FLAG};
use oot_game::collision_check::*;
use oot_game::pack::{BakeBody, BakeSegment, LimbOverride, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::scene_table::gfx_tex_scroll;
use oot_game::skelanime_std::{ANIMMODE_LOOP, ANIMMODE_ONCE, Anim, LimbDraw, SkelAnimeStd, draw_opa_pose};
use oot_game::sys_matrix::MtxF;

use crate::en_bili::{ACTOR_EN_BILI, ACTOR_FLAG_IGNORE_QUAKE, DROP_TABLE_5, EN_BILI_TYPE_SPAWNED_BY_BARI, NA_SE_EN_BIRI_SPARK};

/// `ACTOR_EN_VALI` (`actor_table.h`: 0x0063).
pub const ACTOR_EN_VALI: i16 = 0x0063;
/// `OBJECT_VALI` (`object_table.h`: 0x0056).
pub const OBJECT: &str = "object_vali";
pub const OBJECT_VALI: i16 = 0x0056;
/// `gBariSkel` (`object_vali.xml`): 28 limbs (`BARI_LIMB_MAX` 29 with the root's translation).
pub const SKEL: &str = "gBariSkel";
/// The lists `EnVali_DrawBody` draws itself (`object_vali.xml`).
pub const CORE_DL: &str = "gBariCoreDL";
pub const NUCLEUS_DL: &str = "gBariNucleusDL";
pub const HOOD_DL: &str = "gBariHoodDL";

/// The animations (`object_vali.xml`; `gBariAgitatedAnim` is unused).
pub const ANIM_ATTACK: &str = "gBariAttackAnim";
pub const ANIM_DAMAGED: &str = "gBariDamagedAnim";
pub const ANIM_RECOVER: &str = "gBariRecoverAnim";
pub const ANIM_IDLE: &str = "gBariIdleAnim";

/// `En_Vali_Profile`: `ACTORCAT_ENEMY`, attention-enabled, hostile, always updating, shaken with
/// the view; `OBJECT_VALI`.
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_EN_VALI,
    name: "En_Vali",
    category: ACTORCAT_ENEMY,
    flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE | ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_IGNORE_QUAKE,
    object: OBJECT,
    schema: &SCHEMA,
};

/// `params`: overwritten at init; its own flag after.
pub const STATE: Field = Field::whole(
    "params",
    Kind::Named(&[(-1, "placed (written 0 at init)"), (0, "its shock just a shock (set by itself)"), (1, "damaged after its shock (set by itself)")]),
    "any value placed is lost (init writes 0); 1 while a sword's shock is to damage it",
);

/// En_Vali's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema { fields: &[STATE], flags: &[], todo: false, objects: &[], texts: &[] };

// The sounds (`include/tables/sfx/enemybank_table.h`).
/// `NA_SE_EN_BARI_SPLIT` (0x3887).
pub const NA_SE_EN_BARI_SPLIT: u16 = 0x3887;
/// `NA_SE_EN_BARI_ROLL` (0x389D).
pub const NA_SE_EN_BARI_ROLL: u16 = 0x389D;
/// `NA_SE_EN_BARI_DEAD` (0x389F).
pub const NA_SE_EN_BARI_DEAD: u16 = 0x389F;
/// `NA_SE_EN_BARI_DAMAGE` (0x38DC).
pub const NA_SE_EN_BARI_DAMAGE: u16 = 0x38DC;

/// `NAVI_ENEMY_BARI` (`actor.h`: 0x18).
pub const NAVI_ENEMY_BARI: u8 = 0x18;

/// `EnValiDamageReaction`.
pub const EN_VALI_DMG_REACT_NONE: u8 = 0x0;
pub const EN_VALI_DMG_REACT_STUN: u8 = 0x1;
pub const EN_VALI_DMG_REACT_FIRE: u8 = 0x2;
pub const EN_VALI_DMG_REACT_ICE: u8 = 0x3;
pub const EN_VALI_DMG_REACT_SLINGSHOT: u8 = 0xE;
pub const EN_VALI_DMG_REACT_SWORD_SLASH: u8 = 0xF;

/// `BariLimb` (1-based, `object_vali.xml`'s limbs in order).
pub const BARI_LIMB_NUCLEUS: usize = 2;
pub const BARI_LIMB_LEFT_TENTACLE_ROOT: usize = 9;
pub const BARI_LIMB_LEFT_TENTACLE_BODY_ROOT: usize = 11;
pub const BARI_LIMB_RIGHT_TENTACLE_ROOT: usize = 18;
pub const BARI_LIMB_RIGHT_TENTACLE_BODY_ROOT: usize = 20;
pub const BARI_LIMB_CORE: usize = 27;
pub const BARI_LIMB_HOOD: usize = 28;

/// The colliders' ids.
pub const COL_CYLINDER: u8 = 0;
pub const COL_LEFT_TENTACLE: u8 = 1;
pub const COL_RIGHT_TENTACLE: u8 = 2;

/// `sColliderQuadInit`: a tentacle, hurting 8 (`HIT_SPECIAL_EFFECT_7`, no special effect), no sound.
pub const QUAD_INIT: ColliderQuadInit = ColliderQuadInit {
    base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_ON | AT_TYPE_ENEMY, ac_flags: AC_NONE, oc_flags1: OC1_NONE, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_QUAD },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0xFFCF_FFFF, hit_special_effect: HIT_SPECIAL_EFFECT_7, damage: 0x08 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0x0000_0000, hit_backlash: HIT_BACKLASH_NONE, defense: 0x00 },
        at_elem_flags: ATELEM_ON | ATELEM_SFX_NONE,
        ac_elem_flags: ACELEM_NONE,
        oc_elem_flags: OCELEM_NONE,
    },
    quad: [Vec3::ZERO; 4],
};

/// `sCylinderInit`: `COL_MATERIAL_HIT8`; hurting 8 (`HIT_SPECIAL_EFFECT_7`), hit by Player's
/// `0xFFCFFFFF` with an electric backlash, pushing as type 1; 17 by 35, 15 down.
pub const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_HIT8, at_flags: AT_ON | AT_TYPE_ENEMY, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0xFFCF_FFFF, hit_special_effect: HIT_SPECIAL_EFFECT_7, damage: 0x08 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: HIT_BACKLASH_ELECTRIC, defense: 0x00 },
        at_elem_flags: ATELEM_ON | ATELEM_SFX_NORMAL,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 17, height: 35, y_shift: -15, pos: [0; 3] },
};

/// `sColChkInfoInit`: health 2, 18 by 32, `MASS_HEAVY`.
pub const COL_CHK_INFO_INIT: CollisionCheckInfoInit = CollisionCheckInfoInit { health: 2, cyl_radius: 18, cyl_height: 32, mass: MASS_HEAVY };

/// `sDamageTable`.
pub static S_DAMAGE_TABLE: DamageTable = DamageTable {
    table: [
        dmg_entry(0, EN_VALI_DMG_REACT_STUN),        // Deku nut
        dmg_entry(2, EN_VALI_DMG_REACT_NONE),        // Deku stick
        dmg_entry(0, EN_VALI_DMG_REACT_SLINGSHOT),   // Slingshot
        dmg_entry(2, EN_VALI_DMG_REACT_NONE),        // Explosive
        dmg_entry(0, EN_VALI_DMG_REACT_STUN),        // Boomerang
        dmg_entry(2, EN_VALI_DMG_REACT_NONE),        // Normal arrow
        dmg_entry(2, EN_VALI_DMG_REACT_NONE),        // Hammer swing
        dmg_entry(2, EN_VALI_DMG_REACT_NONE),        // Hookshot
        dmg_entry(1, EN_VALI_DMG_REACT_SWORD_SLASH), // Kokiri sword
        dmg_entry(2, EN_VALI_DMG_REACT_SWORD_SLASH), // Master sword
        dmg_entry(4, EN_VALI_DMG_REACT_SWORD_SLASH), // Giant's Knife
        dmg_entry(4, EN_VALI_DMG_REACT_FIRE),        // Fire arrow
        dmg_entry(4, EN_VALI_DMG_REACT_ICE),         // Ice arrow
        dmg_entry(2, EN_VALI_DMG_REACT_NONE),        // Light arrow
        dmg_entry(2, EN_VALI_DMG_REACT_NONE),        // Unk arrow 1
        dmg_entry(2, EN_VALI_DMG_REACT_NONE),        // Unk arrow 2
        dmg_entry(2, EN_VALI_DMG_REACT_NONE),        // Unk arrow 3
        dmg_entry(4, EN_VALI_DMG_REACT_FIRE),        // Fire magic
        dmg_entry(4, EN_VALI_DMG_REACT_ICE),         // Ice magic
        dmg_entry(0, EN_VALI_DMG_REACT_NONE),        // Light magic
        dmg_entry(0, EN_VALI_DMG_REACT_NONE),        // Shield
        dmg_entry(0, EN_VALI_DMG_REACT_NONE),        // Mirror Ray
        dmg_entry(1, EN_VALI_DMG_REACT_NONE),        // Kokiri spin
        dmg_entry(4, EN_VALI_DMG_REACT_NONE),        // Giant spin
        dmg_entry(2, EN_VALI_DMG_REACT_NONE),        // Master spin
        dmg_entry(2, EN_VALI_DMG_REACT_NONE),        // Kokiri jump
        dmg_entry(8, EN_VALI_DMG_REACT_NONE),        // Giant jump
        dmg_entry(4, EN_VALI_DMG_REACT_NONE),        // Master jump
        dmg_entry(0, EN_VALI_DMG_REACT_NONE),        // Unknown 1
        dmg_entry(0, EN_VALI_DMG_REACT_NONE),        // Unblockable
        dmg_entry(4, EN_VALI_DMG_REACT_NONE),        // Hammer jump
        dmg_entry(0, EN_VALI_DMG_REACT_NONE),        // Unknown 2
    ],
};

/// The segments `EnVali_Draw` binds: the core's scroll, its combiner; segment 1 (the nucleus's
/// `play->billboardMtx`): the identity, the draw's matrix multiplied by the billboard itself.
const SEG_SCROLL: u8 = 0x08;
const SEG_COMBINER: u8 = 0x09;
const SEG_BILLBOARD: u8 = 0x01;
/// `sBariCoreInvertTexCombinerDL` and `sBariCoreNormalCombinerDL`: `gsDPSetCombineLERP(1 or
/// TEXEL0, TEXEL0 or 0, SHADE, 0, TEXEL0, 0, PRIMITIVE, 0, COMBINED, 0, PRIMITIVE, 0, TEXEL1, 0,
/// PRIM_LOD_FRAC, COMBINED)` and `gsSPEndDisplayList()` (the same words as `En_Bili`'s).
pub const INVERT_COMBINER_DL: [(u32, u32); 2] = [(0xFC62_1603, 0x1F5B_FFF8), (0xDF00_0000, 0)];
pub const NORMAL_COMBINER_DL: [(u32, u32); 2] = [(0xFC12_1603, 0xFF5B_FFF8), (0xDF00_0000, 0)];

/// The bakes.
pub const BAKE_SKEL: &str = "En_Vali/skel";
pub const BAKE_NUCLEUS: &str = "En_Vali/nucleus";
pub const BAKE_HOOD: &str = "En_Vali/hood";

/// The core's bake with its combiner inverted or not.
pub fn core_bake_name(invert: bool) -> &'static str {
    if invert { "En_Vali/core_invert" } else { "En_Vali/core_normal" }
}

/// `Gfx_TexScroll(gfxCtx, 0, (0x7F - gameplayFrames × 12) & 0x7F, 32, 32)`.
pub fn core_scroll(frames: u32) -> Vec<(u32, u32)> {
    gfx_tex_scroll(0, 0x7Fu32.wrapping_sub(frames.wrapping_mul(12)) & 0x7F, 32, 32)
}

/// `EnVali_Draw`'s meshes.
pub fn bakes() -> Vec<MeshBake> {
    let dl = |name: &str, symbol: &str, segments: Vec<(u8, BakeSegment)>| MeshBake { name: name.into(), object: OBJECT.into(), segments, prelude: vec![], body: BakeBody::DLists(vec![(OBJECT.into(), symbol.into())]) };
    let mut v = Vec::new();
    for invert in [false, true] {
        let combiner = if invert { INVERT_COMBINER_DL } else { NORMAL_COMBINER_DL };
        v.push(dl(core_bake_name(invert), CORE_DL, vec![(SEG_SCROLL, BakeSegment::Dynamic(core_scroll(0))), (SEG_COMBINER, BakeSegment::Commands(combiner.to_vec()))]));
    }
    v.push(dl(BAKE_NUCLEUS, NUCLEUS_DL, vec![(SEG_BILLBOARD, BakeSegment::Bytes(crate::en_item00::identity_mtx()))]));
    v.push(dl(BAKE_HOOD, HOOD_DL, vec![]));
    // EnVali_OverrideLimbDraw: the nucleus, the hood and the core drawn by EnVali_DrawBody.
    let nulled = [BARI_LIMB_NUCLEUS, BARI_LIMB_HOOD, BARI_LIMB_CORE];
    v.push(MeshBake {
        name: BAKE_SKEL.into(),
        object: OBJECT.into(),
        segments: vec![],
        prelude: vec![],
        body: BakeBody::Skeleton { file: OBJECT.into(), symbol: SKEL.into(), limbs: nulled.iter().map(|&l| LimbOverride { limb: (l - 1) as u8, file: OBJECT.into(), symbol: String::new() }).collect() },
    });
    v
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    WaitPlayerNear,
    Appear,
    Spin,
    Electrify,
    Damaged,
    Recover,
    Burnt,
    Die,
    Stunned,
    Frozen,
    Bury,
}

impl Action {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Action::WaitPlayerNear => "EnVali_WaitPlayerNear",
            Action::Appear => "EnVali_Appear",
            Action::Spin => "EnVali_Spin",
            Action::Electrify => "EnVali_Electrify",
            Action::Damaged => "EnVali_Damaged",
            Action::Recover => "EnVali_Recover",
            Action::Burnt => "EnVali_Burnt",
            Action::Die => "EnVali_Die",
            Action::Stunned => "EnVali_Stunned",
            Action::Frozen => "EnVali_Frozen",
            Action::Bury => "EnVali_Bury",
        }
    }
}

struct Anims {
    attack: Anim,
    damaged: Anim,
    recover: Anim,
    idle: Anim,
}

impl Anims {
    fn load(play: &PlayState) -> Option<Anims> {
        let a = play.assets.clone()?;
        let get = |s: &str| a.animation(OBJECT, s).map_err(|e| log::error!("En_Vali: {e:#}")).ok();
        Some(Anims { attack: get(ANIM_ATTACK)?, damaged: get(ANIM_DAMAGED)?, recover: get(ANIM_RECOVER)?, idle: get(ANIM_IDLE)? })
    }
}

pub struct EnVali {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    skeleton: Option<Arc<Skeleton>>,
    anims: Option<Anims>,
    pub action: Action,
    /// `electrifyTimer` (`u8`).
    pub electrify_timer: u8,
    /// `spinTimer` (`u8`).
    pub spin_timer: u8,
    pub timer: i16,
    pub tentacles_length_factor: f32,
    pub spin_height: f32,
    pub left_tentacle_collider: ColliderQuad,
    pub right_tentacle_collider: ColliderQuad,
    pub collider: ColliderCylinder,
    /// `actor.draw` set (`EnVali_Draw`), not `NULL` (`EnVali_SetupWaitPlayerNear`, `_SetupDie`).
    pub drawn: bool,
    /// The length factors the last draw stretched the left and the right tentacle by.
    pub drawn_factors: [f32; 2],
}

/// `DEG_TO_BINANG(degrees)`: `(s16)(s32)(degrees × (0x8000 / 180.0f))`.
fn deg_to_binang(degrees: f32) -> i16 {
    (degrees * (32768.0f32 / 180.0)) as i32 as i16
}

/// `EnVali_GetHoodScale`: the hood's scale for `action` at the animation's `frame`
/// (`electrify_timer` shocking, `timer` stunned).
pub fn hood_scale(action: Action, frame: f32, electrify_timer: u8, timer: i16) -> Vec3 {
    use std::f64::consts::PI;
    let mut scale = Vec3::ONE;
    match action {
        Action::Electrify => {
            let mut n = 20 - (electrify_timer as i32 % 20);
            if n >= 10 {
                n -= 10;
            }
            scale.y -= 0.2 * ((n as f64 * (PI / 10.0)) as f32).sin();
        }
        Action::Damaged => {
            let f = (((PI / 10.0) * frame as f64) as f32).sin();
            scale.y -= 0.24 * f;
            scale.x -= 0.13 * f;
            scale.z = scale.x;
        }
        Action::Recover => {
            let f = (((PI / 50.0) * frame as f64) as f32).cos();
            scale.y -= 0.24 * f;
            scale.x -= 0.13 * f;
            scale.z = scale.x;
        }
        Action::Stunned => {
            let f = ((timer as f64 * (PI / 10.0)) as f32).sin() * 0.08;
            scale.x += f;
            scale.y -= f;
            scale.z += f;
        }
        _ => {
            let frame = if frame >= 40.0 { frame - 40.0 } else { frame };
            scale.y -= 0.2 * (((PI / 40.0) * frame as f64) as f32).sin();
        }
    }
    scale
}

/// `EnVali_GetCoreScale`.
pub fn core_scale(action: Action, frame: f32, electrify_timer: u8, timer: i16) -> Vec3 {
    use std::f64::consts::PI;
    let mut scale = Vec3::ONE;
    match action {
        Action::Electrify => {
            let mut n = 0x14 - (electrify_timer as i32 % 20);
            if n >= 0xA {
                n -= 0xA;
            }
            scale.y -= 0.13 * ((n as f64 * (PI / 10.0)) as f32).sin();
        }
        Action::Damaged => {
            let f = (((PI / 10.0) * frame as f64) as f32).sin();
            scale.y -= 0.18 * f;
            scale.x -= 0.1 * f;
            scale.z = scale.x;
        }
        Action::Recover => {
            let f = (((PI / 50.0) * frame as f64) as f32).cos();
            scale.y -= 0.18 * f;
            scale.x -= 0.1 * f;
            scale.z = scale.x;
        }
        Action::Stunned => {
            let f = ((timer as f64 * (PI / 10.0)) as f32).sin() * 0.08;
            scale.x -= f;
            scale.y += f;
            scale.z -= f;
        }
        _ => {
            let frame = if frame >= 40.0 { frame - 40.0 } else { frame };
            scale.y -= 0.13 * (((PI / 40.0) * frame as f64) as f32).sin();
        }
    }
    scale
}

/// `EnVali_UpdateTentaclesLengthFactor`'s target for `action` at the animation's `frame` (the
/// factor itself, shocking or frozen).
pub fn tentacles_length_target(action: Action, frame: f32, factor: f32) -> f32 {
    use std::f64::consts::PI;
    match action {
        Action::Spin => {
            if frame <= 10.0 {
                (frame * 0.05) + 1.0
            } else if frame > 70.0 {
                ((80.0 - frame) * 0.05) + 1.0
            } else {
                1.5
            }
        }
        Action::Damaged => 1.0 - ((((PI / 10.0) * frame as f64) as f32).sin() * 0.35),
        Action::Recover => 1.0 - ((((PI / 50.0) * frame as f64) as f32).cos() * 0.35),
        Action::Electrify | Action::Frozen => factor,
        _ => 1.0,
    }
}

impl EnVali {
    /// `EnVali_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // sInitChain: naviEnemyId NAVI_ENEMY_BARI, scale 10 / 1000, lockOnArrowOffset 5000.
        actor.navi_enemy_id = NAVI_ENEMY_BARI;
        actor.scale = Vec3::splat(0.01);
        actor.target_arrow_offset = 5000.0;
        actor.shape_init(0.0, oot_game::shadow::ShadowDraw::Circle, 27.0);
        actor.shape_shadow_alpha = 155;
        let anims = Anims::load(play);
        let skeleton = play.assets.clone().and_then(|a| a.skeleton(OBJECT, SKEL).map_err(|e| log::error!("En_Vali: {e:#}")).ok());
        // SkelAnime_Init(.., &gBariSkel, &gBariIdleAnim, .., BARI_LIMB_MAX).
        let skel = SkelAnimeStd::init_flex(skeleton.as_ref().map(|s| s.limbs.len()).unwrap_or(28), anims.as_ref().map(|a| a.idle.clone()));
        actor.col_chk_info.set_info(Some(&S_DAMAGE_TABLE), &COL_CHK_INFO_INIT);
        let mut this = EnVali {
            actor,
            skel,
            skeleton,
            anims,
            action: Action::WaitPlayerNear,
            electrify_timer: 0,
            spin_timer: 0,
            timer: 0,
            tentacles_length_factor: 0.0,
            spin_height: 0.0,
            left_tentacle_collider: ColliderQuad::new(&QUAD_INIT),
            right_tentacle_collider: ColliderQuad::new(&QUAD_INIT),
            collider: ColliderCylinder::new(&CYLINDER_INIT),
            drawn: true,
            drawn_factors: [1.0; 2],
        };
        this.setup_wait_player_near();
        this.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        // BgCheck_EntityRaycastDown4 from its place.
        let (floor_height, floor_poly) = play.col.entity_raycast_down(this.actor.world_pos);
        this.actor.floor_height = floor_height;
        this.actor.floor_poly = floor_poly;
        // schema: params written whole (0)
        this.actor.params = 0;
        if this.actor.floor_height == eng_collision::bgcheck::BGCHECK_Y_MIN {
            this.actor.kill();
        }
        Box::new(this)
    }

    fn anim(&self, f: impl Fn(&Anims) -> &Anim) -> Option<Anim> {
        self.anims.as_ref().map(|a| f(a).clone())
    }

    /// `Animation_Change(anim, 1, 0, lastFrame, mode, morphFrames)`.
    fn change(&mut self, f: impl Fn(&Anims) -> &Anim, mode: u8, morph_frames: f32) {
        if let Some(a) = self.anim(f) {
            let last = a.last_frame();
            self.skel.change(a, 1.0, 0.0, last, mode, morph_frames);
        }
    }

    /// `EnVali_SetupWaitPlayerNear`: `gBariIdleAnim` looped, not drawn, not hittable.
    pub fn setup_wait_player_near(&mut self) {
        // Animation_PlayLoop.
        self.change(|a| &a.idle, ANIMMODE_LOOP, 0.0);
        self.drawn = false;
        self.collider.base.ac_flags &= !AC_ON;
        self.action = Action::WaitPlayerNear;
    }

    /// `EnVali_SetupAppear`: drawn, lock-on on, falling at 1.
    pub fn setup_appear(&mut self) {
        self.drawn = true;
        self.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED;
        self.action = Action::Appear;
        self.actor.velocity.y = 1.0;
    }

    /// `EnVali_SetupSpin`: `gBariAttackAnim` looped (morphing 3), both quads' corners 10 under it,
    /// updating only near, hittable, `spinHeight` its height.
    pub fn setup_spin(&mut self) {
        // Animation_MorphToLoop(.., -3.0f).
        self.change(|a| &a.attack, ANIMMODE_LOOP, -3.0);
        let mut p = self.actor.world_pos;
        p.y = self.actor.world_pos.y - 10.0;
        self.left_tentacle_collider.dim.quad = [p; 4];
        self.right_tentacle_collider.dim.quad = [p; 4];
        self.actor.flags &= !ACTOR_FLAG_UPDATE_CULLING_DISABLED;
        self.collider.base.ac_flags |= AC_ON;
        self.spin_timer = 0;
        self.action = Action::Spin;
        self.spin_height = self.actor.world_pos.y;
    }

    /// `EnVali_SetupElectrify`: 20 frames, no lock-on, not hittable.
    pub fn setup_electrify(&mut self) {
        self.electrify_timer = 20;
        self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        self.collider.base.ac_flags &= !AC_ON;
        self.action = Action::Electrify;
    }

    /// `EnVali_SetupDamaged`: `gBariDamagedAnim` once (morphing 5), red 30 frames, `params` 0,
    /// not hittable.
    pub fn setup_damaged(&mut self) {
        // Animation_MorphToPlayOnce(.., -5.0f).
        self.change(|a| &a.damaged, ANIMMODE_ONCE, -5.0);
        self.actor.set_color_filter(COLORFILTER_COLORFLAG_RED, 150, COLORFILTER_BUFFLAG_XLU, 30);
        // schema: params written whole (0)
        self.actor.params = 0;
        self.collider.base.ac_flags &= !AC_ON;
        self.action = Action::Damaged;
    }

    /// `EnVali_SetupRecover`: `gBariRecoverAnim` once.
    fn setup_recover(&mut self) {
        if let Some(a) = self.anim(|a| &a.recover) {
            self.skel.play_once(a);
        }
        self.action = Action::Recover;
    }

    /// `EnVali_SetupBurnt`: 2 frames, not hittable, red 30 frames.
    fn setup_burnt(&mut self) {
        self.timer = 2;
        self.collider.base.ac_flags &= !AC_ON;
        self.actor.set_color_filter(COLORFILTER_COLORFLAG_RED, 150, COLORFILTER_BUFFLAG_XLU, 30);
        self.action = Action::Burnt;
    }

    /// `EnVali_SetupDie`: see the module's docs.
    pub fn setup_die(&mut self, play: &mut PlayState) {
        let p = self.actor.world_pos;
        for _ in 0..3 {
            let _ = play.actor_spawn(ACTOR_EN_BILI, p, [0, self.actor.world_rot.y, 0], EN_BILI_TYPE_SPAWNED_BY_BARI);
            self.actor.world_rot.y = self.actor.world_rot.y.wrapping_add(0x5555);
        }
        crate::en_item00::item_drop_collectible_random(play, Some(&self.actor), self.actor.world_pos, DROP_TABLE_5);
        self.timer = play.rand.s16_offset(10, 10);
        self.collider.base.ac_flags &= !AC_ON;
        play.sfx_source_play_sfx_at_fixed_world_pos(self.actor.world_pos, 40, NA_SE_EN_BARI_SPLIT);
        self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        self.drawn = false;
        self.action = Action::Die;
    }

    /// `EnVali_SetupStunned`: `gBariAttackAnim` once (morphing 10), 80 frames, blue 80 frames,
    /// no backlash, `NA_SE_EN_GOMA_JR_FREEZE`, falling from 1.
    fn setup_stunned(&mut self, play: &mut PlayState) {
        // Animation_MorphToPlayOnce(.., 10.0f).
        self.change(|a| &a.attack, ANIMMODE_ONCE, 10.0);
        self.timer = 80;
        self.actor.velocity.y = 0.0;
        self.actor.set_color_filter(COLORFILTER_COLORFLAG_BLUE, 255, COLORFILTER_BUFFLAG_XLU, 80);
        self.collider.info.ac_dmg_info.hit_backlash = HIT_BACKLASH_NONE;
        audio_play_actor_sfx2(play, NA_SE_EN_GOMA_JR_FREEZE);
        self.action = Action::Stunned;
        self.actor.velocity.y = 1.0;
    }

    /// `EnVali_SetupFrozen`: still, blue 36 frames, not hittable, 36 frames.
    fn setup_frozen(&mut self) {
        self.actor.velocity.y = 0.0;
        self.actor.set_color_filter(COLORFILTER_COLORFLAG_BLUE, 255, COLORFILTER_BUFFLAG_XLU, 36);
        self.collider.base.ac_flags &= !AC_ON;
        self.timer = 36;
        self.action = Action::Frozen;
    }

    /// `EnVali_SetupBury`: `gBariIdleAnim` once (morphing 10), always updating, no lock-on.
    pub fn setup_bury(&mut self) {
        // Animation_MorphToPlayOnce(.., 10.0f).
        self.change(|a| &a.idle, ANIMMODE_ONCE, 10.0);
        self.actor.flags |= ACTOR_FLAG_UPDATE_CULLING_DISABLED;
        self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        self.action = Action::Bury;
    }

    /// `EnVali_DoElectricEffectsAndSfx`: on each frame of the four corners a bolt 12 round it
    /// across the camera's view, and the spark's sound.
    fn do_electric_effects_and_sfx(&mut self, play: &mut PlayState) {
        // sEffPrimColor, sEffEnvColor.
        const EFF_PRIM_COLOR: [u8; 4] = [255, 255, 255, 255];
        const EFF_ENV_COLOR: [u8; 4] = [200, 255, 255, 255];
        for i in 0..4i32 {
            let cam_yaw = play.cam_dir_yaw();
            let cos_cam_yaw = -cos_s(cam_yaw);
            let sin_cam_yaw = sin_s(cam_yaw);
            if (self.electrify_timer as i32 + (i << 1)) % 4 == 0 {
                let angle = ((play.rand.centered_float(12288.0) as i32 as i16) as i32 + (i * 0x4000) + 0x2000) as i16;
                let w = self.actor.world_pos;
                let pos = Vec3::new(w.x + (sin_s(angle) * 12.0 * cos_cam_yaw), w.y - (cos_s(angle) * 12.0) + 10.0, w.z + (sin_s(angle) * 12.0 * sin_cam_yaw));
                play.with_ss(|s| s.lightning_spawn(pos, EFF_PRIM_COLOR, EFF_ENV_COLOR, 17, angle, 6, 2));
            }
        }
        self.actor.play_sfx_flagged(NA_SE_EN_BIRI_SPARK - SFX_FLAG);
    }

    /// `EnVali_WaitPlayerNear`: Link within 150 across, it drops.
    fn wait_player_near(&mut self) {
        if self.actor.xz_dist_to_player < 150.0 {
            self.setup_appear();
        }
    }

    /// `EnVali_Appear`: falling 1.5 times faster each frame (40 at most) to the floor.
    fn appear(&mut self, play: &mut PlayState) {
        self.skel.update();
        self.actor.velocity.y *= 1.5;
        self.actor.velocity.y = self.actor.velocity.y.min(40.0);
        if step_to_f(&mut self.actor.world_pos.y, self.actor.floor_height, self.actor.velocity.y) {
            audio_play_actor_sfx2(play, NA_SE_EN_DODO_M_GND);
            self.setup_spin();
        }
    }

    /// `EnVali_Spin`: see the module's docs.
    fn spin(&mut self, play: &mut PlayState) {
        self.skel.update();
        if self.spin_timer != 0 {
            self.spin_timer -= 1;
        }
        let mut anim_cur_frame = self.skel.cur_frame as i32;
        step_to_f(&mut self.spin_height, self.actor.floor_height + 40.0, 1.2);
        // sinf(animCurFrame * M_PI * 0.0125f): the product in double.
        self.actor.world_pos.y = self.spin_height - (((anim_cur_frame as f64 * std::f64::consts::PI * 0.0125f32 as f64) as f32).sin() * 8.0);
        if self.spin_timer != 0 {
            self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(0x800);
            if self.spin_timer % 6 == 0 && anim_cur_frame >= 16 && anim_cur_frame < 56 {
                audio_play_actor_sfx2(play, NA_SE_EN_BARI_ROLL);
            }
        } else if matches!(anim_cur_frame, 16 | 30 | 42 | 55) {
            audio_play_actor_sfx2(play, NA_SE_EN_BARI_ROLL);
        }
        if anim_cur_frame > 40 {
            anim_cur_frame = 80 - anim_cur_frame;
        }
        self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(deg_to_binang((anim_cur_frame + 4) as f32 * 0.4));
        if self.actor.xz_dist_to_player > 250.0 {
            self.setup_bury();
        }
    }

    /// `EnVali_Electrify`: 20 frames shaking, then spinning or damaged.
    fn electrify(&mut self, play: &mut PlayState) {
        if self.electrify_timer != 0 {
            self.electrify_timer -= 1;
        }
        self.do_electric_effects_and_sfx(play);
        if self.electrify_timer == 0 {
            self.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED;
            self.collider.base.ac_flags |= AC_ON;
            if STATE.get(self.actor.params) == 1 {
                self.setup_damaged();
            } else {
                self.action = Action::Spin;
            }
        } else if self.electrify_timer & 1 != 0 {
            self.actor.world_pos.y += 1.0;
        } else {
            self.actor.world_pos.y -= 1.0;
        }
    }

    /// `EnVali_Damaged`: the animation done, recovering, or with no health dying.
    fn damaged(&mut self, play: &mut PlayState) {
        if self.skel.update() {
            if self.actor.col_chk_info.health != 0 {
                self.setup_recover();
            } else {
                self.setup_die(play);
            }
        }
    }

    /// `EnVali_Recover`: the animation done, spinning.
    fn recover(&mut self) {
        if self.skel.update() {
            self.setup_spin();
        }
    }

    /// `EnVali_Burnt`: 2 frames, then dying.
    fn burnt(&mut self, play: &mut PlayState) {
        if self.timer != 0 {
            self.timer -= 1;
        }
        if self.timer == 0 {
            self.setup_die(play);
        }
    }

    /// `EnVali_Die`: two bubbles a frame till its count's out, then gone.
    fn die(&mut self, play: &mut PlayState) {
        // sEffVel (its y written each time), sEffAccel.
        let eff_accel = Vec3::ZERO;
        if self.timer != 0 {
            self.timer -= 1;
        }
        for _ in 0..2 {
            let w = self.actor.world_pos;
            let x = play.rand.centered_float(20.0) + w.x;
            let y = play.rand.centered_float(8.0) + w.y;
            let z = play.rand.centered_float(20.0) + w.z;
            let eff_pos = Vec3::new(x, y, z);
            let eff_vel = Vec3::new(0.0, play.rand.zero_one() + 1.0, 0.0);
            let eff_scale = play.rand.s16_offset(40, 40);
            let profile = if play.rand.zero_one() < 0.7 { 2 } else { 0 };
            play.with_ss(|s| s.dt_bubble_spawn_color_profile(eff_pos, eff_vel, eff_accel, eff_scale, 25, profile, 1));
        }
        if self.timer == 0 {
            self.actor.kill();
        }
    }

    /// `EnVali_Stunned`: falling to the floor, 80 frames, then its backlash back and spinning.
    fn stunned(&mut self, play: &mut PlayState) {
        self.skel.update();
        if self.timer != 0 {
            self.timer -= 1;
        }
        if self.actor.velocity.y != 0.0 {
            if step_to_f(&mut self.actor.world_pos.y, self.actor.floor_height, self.actor.velocity.y) {
                self.actor.velocity.y = 0.0;
                audio_play_actor_sfx2(play, NA_SE_EN_DODO_M_GND);
            } else {
                self.actor.velocity.y += 1.0;
            }
        }
        if self.timer == 0 {
            self.collider.info.ac_dmg_info.hit_backlash = HIT_BACKLASH_ELECTRIC;
            self.setup_spin();
        }
    }

    /// `EnVali_Frozen`: see the module's docs.
    fn frozen(&mut self, play: &mut PlayState) {
        if self.timer != 0 {
            self.timer -= 1;
        }
        let temp_v1 = self.timer as i32 - 20;
        self.actor.color_filter_timer = 36;
        if temp_v1 > 0 {
            let temp_v0_2 = temp_v1 >> 1;
            if self.timer & 1 != 0 {
                let w = self.actor.world_pos;
                let y = (w.y - 20.0) + ((-temp_v0_2 * 5) + 40) as f32;
                let x = w.x + if temp_v0_2 & 2 != 0 { 12.0 } else { -12.0 };
                let z = w.z + if temp_v0_2 & 1 != 0 { 12.0 } else { -12.0 };
                let scale = (play.rand.zero_one() * 0.2) + 1.3;
                let r = self.actor.shape_rot;
                let me = play.cur_actor.map(|h| oot_game::effect::SsActor { handle: h, world_pos: w, shape_rot: [r.x, r.y, r.z] });
                play.effect_ss_en_ice_spawn_flying_vec3f(me, Vec3::new(x, y, z), [150, 150, 150, 250], [235, 245, 255], scale);
            }
        } else if self.timer == 0 {
            self.actor.velocity.y += 1.0;
            if step_to_f(&mut self.actor.world_pos.y, self.actor.floor_height, self.actor.velocity.y) {
                self.setup_die(play);
                self.actor.color_filter_timer = 0;
            }
        }
    }

    /// `EnVali_Bury`: back to its place by halves; there, waiting.
    fn bury(&mut self) {
        self.skel.update();
        if smooth_step_to_f(&mut self.actor.world_pos.y, self.actor.home_pos.y, 0.5, 15.0, 0.1) < 0.01 {
            self.setup_wait_player_near();
        }
    }

    /// `EnVali_CheckCollide`: see the module's docs.
    fn check_collide(&mut self, play: &mut PlayState) {
        if self.collider.base.ac_flags & AC_HIT != 0 {
            self.collider.base.ac_flags &= !AC_HIT;
            let elem = self.collider.info;
            self.actor.set_drop_flag(&elem, true);
            let reaction = self.actor.col_chk_info.damage_reaction;
            if reaction != EN_VALI_DMG_REACT_NONE || self.actor.col_chk_info.damage != 0 {
                if self.actor.apply_damage() == 0 {
                    audio_play_actor_sfx2(play, NA_SE_EN_BARI_DEAD);
                    enemy_start_finishing_blow(play, &self.actor);
                    self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
                } else if reaction != EN_VALI_DMG_REACT_STUN && reaction != EN_VALI_DMG_REACT_SLINGSHOT {
                    audio_play_actor_sfx2(play, NA_SE_EN_BARI_DAMAGE);
                }
                if reaction == EN_VALI_DMG_REACT_STUN {
                    if self.action != Action::Stunned {
                        self.setup_stunned(play);
                    }
                } else if reaction == EN_VALI_DMG_REACT_SWORD_SLASH {
                    if self.action != Action::Stunned {
                        self.actor.set_color_filter(COLORFILTER_COLORFLAG_RED, 150, COLORFILTER_BUFFLAG_XLU, 30);
                        // schema: params written whole (1)
                        self.actor.params = 1;
                        self.setup_electrify();
                    } else {
                        self.setup_damaged();
                    }
                } else if reaction == EN_VALI_DMG_REACT_FIRE {
                    self.setup_burnt();
                } else if reaction == EN_VALI_DMG_REACT_ICE {
                    self.setup_frozen();
                } else if reaction == EN_VALI_DMG_REACT_SLINGSHOT {
                    if self.spin_timer == 0 {
                        self.spin_timer = 20;
                    }
                } else {
                    self.setup_damaged();
                }
            }
        }
    }

    fn run_action(&mut self, play: &mut PlayState) {
        match self.action {
            Action::WaitPlayerNear => self.wait_player_near(),
            Action::Appear => self.appear(play),
            Action::Spin => self.spin(play),
            Action::Electrify => self.electrify(play),
            Action::Damaged => self.damaged(play),
            Action::Recover => self.recover(),
            Action::Burnt => self.burnt(play),
            Action::Die => self.die(play),
            Action::Stunned => self.stunned(play),
            Action::Frozen => self.frozen(play),
            Action::Bury => self.bury(),
        }
    }

    /// The scales `EnVali_DrawBody` draws the core and the hood with.
    pub fn body_scales(&self) -> [Vec3; 2] {
        let f = self.skel.cur_frame;
        [core_scale(self.action, f, self.electrify_timer, self.timer), hood_scale(self.action, f, self.electrify_timer, self.timer)]
    }

    /// `Actor_Draw`'s model matrix, moved by the main camera's quake offset
    /// (`ACTOR_FLAG_IGNORE_QUAKE`).
    fn actor_mtx(pos: Vec3, rot: [i16; 3], scale: Vec3, y_offset: f32, flags: u32, quake: Vec3) -> MtxF {
        let y_offset = y_offset * scale.y;
        let t = if flags & ACTOR_FLAG_IGNORE_QUAKE != 0 { Vec3::new(pos.x + quake.x, pos.y + (y_offset + quake.y), pos.z + quake.z) } else { Vec3::new(pos.x, pos.y + y_offset, pos.z) };
        let mut m = MtxF::set_translate_rotate_yxz(t.x, t.y, t.z, rot);
        m.scale(scale.x, scale.y, scale.z);
        m
    }

    /// `EnVali_DrawBody`'s three nuclei's matrices from the model's (`mf`), before the billboard.
    pub fn nucleus_matrices(mf: &MtxF, yaw: i16) -> [Mat4; 3] {
        let mut m = *mf;
        // Matrix_RotateY(BINANG_TO_RAD(-shape.rot.y), MTXMODE_APPLY).
        m.rotate_y((-(yaw as i32)) as f32 * (std::f32::consts::PI / 32768.0));
        let (cos_yaw, sin_yaw) = (cos_s(yaw), sin_s(yaw));
        let a = m.to_mat4();
        m.translate((506.0 * cos_yaw) + (372.0 * sin_yaw), 1114.0, (372.0 * cos_yaw) - (506.0 * sin_yaw));
        let b = m.to_mat4();
        m.translate((-964.0 * cos_yaw) - (804.0 * sin_yaw), -108.0, (-804.0 * cos_yaw) + (964.0 * sin_yaw));
        [a, b, m.to_mat4()]
    }
}

/// `render_state`'s layout: switches drawn, the inverted combiner, the flags; values the core's
/// and the hood's scales (x, y, z each), the left and right tentacles' length factors.
mod rs {
    pub const CORE: usize = 0;
    pub const HOOD: usize = 3;
    pub const FACTORS: usize = 6;
}

impl ActorImpl for EnVali {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnVali_Update`: see the module's docs.
    fn update(&mut self, play: &mut PlayState) {
        if self.collider.base.at_flags & AT_HIT != 0 || self.left_tentacle_collider.base.at_flags & AT_HIT != 0 || self.right_tentacle_collider.base.at_flags & AT_HIT != 0 {
            self.left_tentacle_collider.base.at_flags &= !AT_HIT;
            self.right_tentacle_collider.base.at_flags &= !AT_HIT;
            self.collider.base.at_flags &= !AT_HIT;
            self.setup_electrify();
        }
        self.check_collide(play);
        self.run_action(play);
        if self.action != Action::Die && self.action != Action::WaitPlayerNear {
            self.collider.update(&self.actor);
            if self.action == Action::Spin {
                play.collision_check_set_at(&self.actor, COL_LEFT_TENTACLE, &mut self.left_tentacle_collider);
                play.collision_check_set_at(&self.actor, COL_RIGHT_TENTACLE, &mut self.right_tentacle_collider);
                play.collision_check_set_at(&self.actor, COL_CYLINDER, &mut self.collider);
            }
            if self.collider.base.ac_flags & AC_ON != 0 {
                play.collision_check_set_ac(&self.actor, COL_CYLINDER, &mut self.collider);
            }
            play.collision_check_set_oc(&self.actor, COL_CYLINDER, &mut self.collider);
            self.actor.set_focus(0.0);
        }
    }

    /// `EnVali_Destroy`: the colliders (nothing to free).
    fn destroy(&mut self, _play: &mut PlayState) {}

    /// `EnVali_Draw`'s writes: the tentacles' length factor (`EnVali_OverrideLimbDraw`, each
    /// tentacle root's: two steps a frame) and, spinning, the quads swept by the tentacles
    /// (`EnVali_PostLimbDraw`).
    fn draw_update(&mut self, play: &mut PlayState) {
        if !self.drawn {
            return;
        }
        let Some(skeleton) = self.skeleton.clone() else { return };
        let a = &self.actor;
        let model = Self::actor_mtx(a.world_pos, [a.shape_rot.x, a.shape_rot.y, a.shape_rot.z], a.scale, a.shape_y_offset, a.flags, play.game_camera.quake_offset);
        let (action, frame) = (self.action, self.skel.cur_frame);
        let mut factor = self.tentacles_length_factor;
        let mut drawn_factors = [1.0f32; 2];
        let mut edges: [Option<(Vec3, Vec3)>; 2] = [None, None];
        // D_80B28970, D_80B2897C: the tip and the base along the limb.
        const TIP: Vec3 = Vec3::new(3000.0, 0.0, 0.0);
        const BASE: Vec3 = Vec3::new(-1000.0, 0.0, 0.0);
        crate::boss_va::walk(
            &skeleton,
            &self.skel.joint_table,
            &model,
            |limb, m, _, _| {
                if limb == BARI_LIMB_LEFT_TENTACLE_ROOT || limb == BARI_LIMB_RIGHT_TENTACLE_ROOT {
                    // EnVali_UpdateTentaclesLengthFactor.
                    let target = tentacles_length_target(action, frame, factor);
                    step_to_f(&mut factor, target, 0.1);
                    if factor != 1.0 {
                        m.scale(factor, 1.0, 1.0);
                    }
                    drawn_factors[(limb == BARI_LIMB_RIGHT_TENTACLE_ROOT) as usize] = factor;
                }
            },
            |limb, m| {
                if action == Action::Spin && (limb == BARI_LIMB_LEFT_TENTACLE_BODY_ROOT || limb == BARI_LIMB_RIGHT_TENTACLE_BODY_ROOT) {
                    edges[(limb == BARI_LIMB_RIGHT_TENTACLE_BODY_ROOT) as usize] = Some((m.mult_vec3f(BASE), m.mult_vec3f(TIP)));
                }
            },
        );
        self.tentacles_length_factor = factor;
        self.drawn_factors = drawn_factors;
        // Collider_SetQuadVertices(quad, base, tip, quad[0], quad[1]).
        for (edge, quad) in edges.iter().zip([&mut self.left_tentacle_collider, &mut self.right_tentacle_collider]) {
            if let Some((base, tip)) = *edge {
                let q = quad.dim.quad;
                quad.set_vertices(base, tip, q[0], q[1]);
            }
        }
    }

    fn render_state(&self) -> RenderState {
        let mut r = RenderState::of(&self.actor);
        r.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        let invert = self.electrify_timer & 1 != 0;
        r.switches = vec![self.drawn as u32, invert as u32, self.actor.flags];
        let [c, h] = self.body_scales();
        r.values = vec![c.x, c.y, c.z, h.x, h.y, h.z, self.drawn_factors[0], self.drawn_factors[1]];
        // The shape's shadow: none with draw NULL (Actor_Draw isn't called).
        if !self.drawn {
            r.shadow = None;
        }
        r
    }

    /// `EnVali_Draw`: see the module's docs.
    fn draw(&self, rs: &RenderState, play: &PlayState, view: &ViewInfo, out: &mut DrawOut) {
        let [drawn, invert, flags] = rs.switches.as_slice() else { return };
        if *drawn == 0 {
            return;
        }
        let v = &rs.values;
        if v.len() < 8 {
            return;
        }
        let s = |i: usize| Vec3::new(v[i], v[i + 1], v[i + 2]);
        let mf = Self::actor_mtx(rs.pos, rs.rot, rs.scale, rs.y_offset, *flags, play.game_camera.quake_offset);
        let model = mf.to_mat4();
        // EnVali_DrawBody: the core (its scroll and combiner), the nuclei, the hood.
        let mut sv = SegmentValues::default();
        sv.read(SEG_SCROLL, &core_scroll(play.gameplay_frames));
        out.xlu.push(DrawCmd { mesh: MeshKey::named(keys::bake(core_bake_name(*invert != 0))), transform: model * Mat4::from_scale(s(rs::CORE)), bones: Vec::new(), params: DrawParams { segments: Some(sv), ..Default::default() } });
        for m in Self::nucleus_matrices(&mf, rs.rot[1]) {
            out.xlu.push(DrawCmd::new(MeshKey::named(keys::bake(BAKE_NUCLEUS)), m * view.billboard));
        }
        out.xlu.push(DrawCmd::new(MeshKey::named(keys::bake(BAKE_HOOD)), model * Mat4::from_scale(s(rs::HOOD))));
        // SkelAnime_Draw into POLY_XLU_DISP: the tentacle roots stretched.
        let (Some(joints), Some(skeleton)) = (&rs.joints, &self.skeleton) else { return };
        let (left, right) = (v[rs::FACTORS], v[rs::FACTORS + 1]);
        let bones = draw_opa_pose(
            skeleton,
            &joints.rot,
            |limb, _, _| match limb {
                BARI_LIMB_LEFT_TENTACLE_ROOT => LimbDraw::Default(Mat4::from_scale(Vec3::new(left, 1.0, 1.0))),
                BARI_LIMB_RIGHT_TENTACLE_ROOT => LimbDraw::Default(Mat4::from_scale(Vec3::new(right, 1.0, 1.0))),
                _ => LimbDraw::Default(Mat4::IDENTITY),
            },
            |_, _| {},
        );
        out.xlu.push(DrawCmd { mesh: MeshKey::named(keys::bake(BAKE_SKEL)), transform: model, bones, params: Default::default() });
    }

    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        match id {
            COL_CYLINDER => Some(ColliderMut::Cylinder(&mut self.collider)),
            COL_LEFT_TENTACLE => Some(ColliderMut::Quad(&mut self.left_tentacle_collider)),
            COL_RIGHT_TENTACLE => Some(ColliderMut::Quad(&mut self.right_tentacle_collider)),
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
