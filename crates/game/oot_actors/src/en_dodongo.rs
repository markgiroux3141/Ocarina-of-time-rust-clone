//! `En_Dodongo` (`ovl_En_Dodongo/z_en_dodongo.c`): the Dodongo, `object_dodongo`'s `gDodongoSkel`
//! at scale 0.01875. Dodongo's Cavern (Master Quest) places three in room 5 (params 0xFFFF).
//!
//! - **Idle** (`EnDodongo_Idle`, `gDodongoWaitAnim`): `Rand_S16Offset(30, 50)` frames, then at the
//!   loop's frame 0 it **walks** (`EnDodongo_Walk`, `gDodongoWalkAnim` at 0.75 its speed, at most
//!   1.5, its speed eased to 1.5): a step (`NA_SE_EN_DODO_J_WALK` and a ring of dust at the foot)
//!   each time the walk crosses its frame 21. With Link within 400 of its **home** it turns to him
//!   (at most 0x1F4 a frame), targetable; with him within 100 across, less than 60 above and its
//!   face within 0x1388 of him it **breathes fire**. Else, untargetable, it wanders: past 150 from
//!   home (or retreating) it turns home; its `Rand_S16Offset(50, 70)` frames out, three times in
//!   ten it walks on (and retreats `Rand_S16Offset(15, 40)`), else it idles.
//! - **The fire** (`EnDodongo_BreatheFire`, `gDodongoBreatheFireAnim`): `NA_SE_EN_DODO_J_BREATH`
//!   frames 2 to 20, `NA_SE_EN_DODO_J_CRY` at 24; frames 29 to 43 `NA_SE_EN_DODO_J_FIRE` and a puff
//!   of fire a frame (`Effect_Ss_D_Fire`, 35 up and 30 ahead, pushed on 2.5 a frame, its alpha
//!   `255 - 10 × (frame - 29)`, fading from `frame - 26` frames left), and past 29 to before 43 the
//!   quad (`HIT_SPECIAL_EFFECT_FIRE`, 16: Link burns) out of its mouth, swept across by
//!   `sinf((frame - 28) × 0.08)` (`EnDodongo_UpdateQuad`, limb 2). Then
//!   `gDodongoAfterBreatheFireAnim` and `Rand_S16Offset(10, 20)` frames' idling.
//! - **Hit** (`EnDodongo_CollisionCheck`, its body spheres; not swallowing, dying or sweeping): a
//!   hit bounced off the head's metal triangles (`AC_HARD`, `COL_MATERIAL_METAL`: `AC_BOUNCED`)
//!   cancels a body hit the same frame. Reaction 1 (a Deku Nut, the boomerang, the hookshot) or
//!   0xF (ice) **stuns** it: blue 80 frames (`EnDodongo_Stunned`, held at frame 25 of the fire;
//!   ice: 36 frames of ice chunks, `EffectSsEnIce` flying, its colour held), then idle (or dead at
//!   no health). Other damage: red 8 frames, then dead at 0 health,
//!   else it **sweeps its tail** (`EnDodongo_SweepTail`): `gDodongoDamageAnim`
//!   (`NA_SE_EN_DODO_J_DAMAGE`), then facing away from Link (`yawTowardsPlayer` 0x4000 or more
//!   off), a sweep the way he is (`gDodongoSweepTailLeftAnim` or `Right` at 2, `NA_SE_EN_DODO_J_TAIL`)
//!   turning `(0xFFFF - |yaw|) / 15` a frame for up to 17 frames (the animation ends first, after
//!   15), its tail's spheres (1 and 2) hurting 8 (`DMG_DEFAULT`, `ATELEM_SFX_WOOD`), dust under
//!   them, a white trail (`EFFECT_BLURE1`) from the tail (limb 15); facing him, or after a sweep,
//!   the fire.
//! - **A bomb** (`EnDodongo_AteBomb`, idle, walking, stunned or after the fire): an explosive of
//!   params 0 with no `parent` within 20, 10, 20 of its mouth: an `En_Bom` becomes its `child`, any
//!   other (a Bomb Flower's bomb) its `parent`, and the explosive's `parent` the Dodongo (so a bomb
//!   in Link's hand, his child, isn't eaten). It **swallows it** (`EnDodongo_SwallowBomb`: the fire
//!   backwards from 35, the bomb held at its mouth with its fuse held, white smoke; at frame 28
//!   `NA_SE_EN_DODO_J_EAT` and the bomb gone; held at 24 for 25 frames; at 10 left the blast inside,
//!   eleven puffs from its first sphere (limb 15), `NA_SE_IT_BOMB_EXPLOSION`, red 8 frames, its body
//!   swelling with the red (limbs 15 and 16 scaled, `EnDodongo_OverrideLimbDraw`) and black smoke
//!   from its mouth) and **dies**. (A free bomb touching its body first, OC, blows: `EnBom_Update`.)
//! - **The death** (`EnDodongo_Death`, `gDodongoDieAnim`, `NA_SE_EN_DODO_J_DEAD`, untargetable):
//!   params 0 smokes to frame 35; then red flashes of 4; `NA_SE_EN_RIZA_DOWN` at 52; the animation
//!   done, an `En_Bom` (`BOMB_BODY`, `shape.rot.z` 6) where it lies with its fuse at 0, and 7 frames
//!   on a drop from table 4 (`Item_DropCollectibleRandom`) and it's gone.
//!
//! The whole overlay is ported. The draw (`EnDodongo_Draw`) is the skeleton (`SkelAnime_DrawOpa`,
//! `gDodongoSkel`'s mesh) with limbs 15 and 16 scaled by `bodyScale`; `draw_update` runs
//! `EnDodongo_PostLimbDraw` (the spheres, the fire's quad, the head's triangles, the mouth and head,
//! the feet, the tail's trail, the ice's places) and the ice. The circle shadow (48) is the shape's
//! (`ActorShape_Init`'s, drawn by `Actor_Draw`).
//!
//! @bug (game) (the C's comment): `EnDodongo_SwallowBomb` writes `((EnBombf*)parent)->timer` for
//! any explosive that isn't an `En_Bom`; a Bombchu (`En_Bom_Chu`, params 0) is one, and the write
//! lands past its end, in the next arena node's pointers: massive memory corruption. No Bombchu is
//! ported: a non-`En_Bombf` here is moved to the mouth and its write logged.
//!
//! Params: `params` whole (`EnDodongoParam`): -1 `EN_DODONGO_NORMAL`, 0 `EN_DODONGO_SMOKE_DEATH`
//! (black smoke from its mouth while it dies); every placement -1.
//! Flags: none.

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::Sphere16;
use eng_gfx::{DrawCmd, MeshKey};
use eng_math::{cos_s, sin_s, smooth_step_to_f, smooth_step_to_s, vec3f_yaw};
use glam::{Mat4, Vec3};
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_ENEMY, ACTORCAT_EXPLOSIVE, ActorHandle, ActorImpl, ActorProfile, actor_spawn_floor_dust_ring, audio_play_actor_sfx2, audio_play_actor_sfx_at};
use oot_game::actor_schema::{ActorSchema, Field, Kind};
use oot_game::audio::sfx::{NA_SE_EN_GOMA_JR_FREEZE, NA_SE_IT_BOMB_EXPLOSION, NA_SE_PL_BODY_HIT, SFX_FLAG};
use oot_game::collision_check::*;
use oot_game::effect::EffectInit;
use oot_game::effect::blure::{EffectBlureInit, EffectBlureInit1};
use oot_game::pack::keys;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::skelanime_std::{ANIMMODE_LOOP, ANIMMODE_ONCE, Anim, LimbDraw, SkelAnimeStd, draw_opa_pose};

use crate::en_bom::{ACTOR_EN_BOM, EnBom};
use crate::en_bombf::EnBombf;

/// `ACTOR_EN_DODONGO` (`actor_table.h`: 0x0012).
pub const ACTOR_EN_DODONGO: i16 = 0x0012;
/// `OBJECT_DODONGO` (`object_table.h`: 0x000C).
pub const OBJECT: &str = "object_dodongo";
/// `gDodongoSkel` (`object_dodongo.xml`): 30 limbs.
pub const SKEL: &str = "gDodongoSkel";

/// The animations (`object_dodongo.xml`).
pub const ANIM_DIE: &str = "gDodongoDieAnim";
pub const ANIM_DAMAGE: &str = "gDodongoDamageAnim";
pub const ANIM_BREATHE_FIRE: &str = "gDodongoBreatheFireAnim";
pub const ANIM_AFTER_BREATHE_FIRE: &str = "gDodongoAfterBreatheFireAnim";
pub const ANIM_SWEEP_TAIL_RIGHT: &str = "gDodongoSweepTailRightAnim";
pub const ANIM_SWEEP_TAIL_LEFT: &str = "gDodongoSweepTailLeftAnim";
pub const ANIM_WAIT: &str = "gDodongoWaitAnim";
pub const ANIM_WALK: &str = "gDodongoWalkAnim";

/// `En_Dodongo_Profile`: `ACTORCAT_ENEMY`, attention-enabled, hostile, always updating,
/// `OBJECT_DODONGO`.
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_EN_DODONGO,
    name: "En_Dodongo",
    category: ACTORCAT_ENEMY,
    flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE | ACTOR_FLAG_UPDATE_CULLING_DISABLED,
    object: OBJECT,
    schema: &SCHEMA,
};

/// `EnDodongoParam` (`z_en_dodongo.h`).
pub const EN_DODONGO_NORMAL: i16 = -1;
pub const EN_DODONGO_SMOKE_DEATH: i16 = 0;

/// `params`: `EnDodongoParam`, compared whole.
pub const PARAMS: Field = Field::whole(
    "params",
    Kind::Named(&[(-1, "EN_DODONGO_NORMAL"), (0, "EN_DODONGO_SMOKE_DEATH")]),
    "0: black smoke from its mouth while it dies (EnDodongo_Death); any other value (-1 as placed) none",
);

/// En_Dodongo's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema { fields: &[PARAMS], flags: &[], todo: false, objects: &[], texts: &[] };

// The sounds (`include/tables/sfx/enemybank_table.h`).
/// `NA_SE_EN_DODO_J_WALK` (0x3800).
pub const NA_SE_EN_DODO_J_WALK: u16 = 0x3800;
/// `NA_SE_EN_DODO_J_CRY` (0x3801).
pub const NA_SE_EN_DODO_J_CRY: u16 = 0x3801;
/// `NA_SE_EN_DODO_J_FIRE` (0x3802).
pub const NA_SE_EN_DODO_J_FIRE: u16 = 0x3802;
/// `NA_SE_EN_DODO_J_DAMAGE` (0x3803).
pub const NA_SE_EN_DODO_J_DAMAGE: u16 = 0x3803;
/// `NA_SE_EN_DODO_J_DEAD` (0x3804).
pub const NA_SE_EN_DODO_J_DEAD: u16 = 0x3804;
/// `NA_SE_EN_DODO_J_BREATH` (0x3858).
pub const NA_SE_EN_DODO_J_BREATH: u16 = 0x3858;
/// `NA_SE_EN_DODO_J_TAIL` (0x3859).
pub const NA_SE_EN_DODO_J_TAIL: u16 = 0x3859;
/// `NA_SE_EN_DODO_J_EAT` (0x385B).
pub const NA_SE_EN_DODO_J_EAT: u16 = 0x385B;
/// `NA_SE_EN_RIZA_DOWN` (0x387A).
pub const NA_SE_EN_RIZA_DOWN: u16 = 0x387A;

/// `NAVI_ENEMY_DODONGO` (`actor.h`: 0x0D).
pub const NAVI_ENEMY_DODONGO: u8 = 0x0D;
/// `COLLECTIBLE_DROP_RANDOM_PARAMS(COLLECTIBLE_DROP_TABLE_4, false)` (`z_en_item00.h`: the table
/// × 16).
pub const DROP_TABLE_4: i16 = 4 * 16;
/// `BOMB_BODY` (`z_en_bom.h`'s `EnBomType`).
pub const BOMB_BODY: i16 = 0;

/// `EnDodongoActionState`: compared by order (`> DODONGO_DEATH`, `>= DODONGO_IDLE`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ActionState {
    SweepTail = 0,
    SwallowBomb = 1,
    Death = 2,
    BreatheFire = 3,
    Idle = 4,
    EndBreatheFire = 5,
    /// `DODONGO_UNUSED`: never set.
    Unused = 6,
    Stunned = 7,
    Walk = 8,
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Idle,
    EndBreatheFire,
    BreatheFire,
    SwallowBomb,
    Walk,
    Stunned,
    Death,
    SweepTail,
}

impl Action {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Action::Idle => "EnDodongo_Idle",
            Action::EndBreatheFire => "EnDodongo_EndBreatheFire",
            Action::BreatheFire => "EnDodongo_BreatheFire",
            Action::SwallowBomb => "EnDodongo_SwallowBomb",
            Action::Walk => "EnDodongo_Walk",
            Action::Stunned => "EnDodongo_Stunned",
            Action::Death => "EnDodongo_Death",
            Action::SweepTail => "EnDodongo_SweepTail",
        }
    }
}

/// `sBodyElementsInit`'s element: hit by Player's `ac` flags (or not), pushing.
const fn body_element(ac_dmg_flags: u32, ac_elem_flags: u8, oc_elem_flags: u8, limb: u8, radius: i16) -> ColliderJntSphElementInit {
    ColliderJntSphElementInit {
        info: ColliderElementInit {
            elem_material: ELEM_MATERIAL_UNK0,
            at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0 },
            ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: ac_dmg_flags, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
            at_elem_flags: ATELEM_NONE,
            ac_elem_flags,
            oc_elem_flags,
        },
        limb,
        model_sphere: Sphere16 { center: [0, 0, 0], radius },
        scale: 100,
    }
}

/// `sBodyElementsInit`: the tail (limbs 15, 14, 13: 17, 15, 10 across), the feet (21, 28: 20,
/// pushing only), the body (6: 35, hit by `0x0D800691`, `OCELEM_UNK3`).
pub const BODY_ELEMENTS: [ColliderJntSphElementInit; 6] = [
    body_element(0xFFCF_FFFF, ACELEM_ON, OCELEM_ON, 15, 17),
    body_element(0xFFCF_FFFF, ACELEM_ON, OCELEM_ON, 14, 15),
    body_element(0xFFCF_FFFF, ACELEM_ON, OCELEM_ON, 13, 10),
    body_element(0x0000_0000, ACELEM_NONE, OCELEM_ON, 21, 20),
    body_element(0x0000_0000, ACELEM_NONE, OCELEM_ON, 28, 20),
    body_element(0x0D80_0691, ACELEM_ON, OCELEM_ON | OCELEM_UNK3, 6, 35),
];

/// `sBodyJntSphInit`: `AT_ON | AT_TYPE_ENEMY` (no element's on until the sweep), hit by
/// Player's, pushing.
pub const BODY_JNT_SPH_INIT: ColliderInit = ColliderInit { col_type: COL_MATERIAL_HIT0, at_flags: AT_ON | AT_TYPE_ENEMY, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_JNTSPH };

/// `sHardElementsInit`'s element (`ELEM_MATERIAL_UNK2`, hookable, no AT info).
const fn hard_element(ac_dmg_flags: u32, vtx: [Vec3; 3]) -> ColliderTrisElementInit {
    ColliderTrisElementInit {
        info: ColliderElementInit {
            elem_material: ELEM_MATERIAL_UNK2,
            at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0 },
            ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: ac_dmg_flags, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
            at_elem_flags: ATELEM_NONE,
            ac_elem_flags: ACELEM_ON | ACELEM_HOOKABLE | ACELEM_NO_AT_INFO,
            oc_elem_flags: OCELEM_NONE,
        },
        vtx,
    }
}

/// `sHardElementsInit`: the head's three triangles (placed by the draw, limb 7).
pub const HARD_ELEMENTS: [ColliderTrisElementInit; 3] = [
    hard_element(0xF24B_F96E, [Vec3::new(-10.0, 14.0, 2.0), Vec3::new(-10.0, -6.0, 2.0), Vec3::new(9.0, 14.0, 2.0)]),
    hard_element(0xFFCB_F96E, [Vec3::new(-10.0, -6.0, 2.0), Vec3::new(9.0, -6.0, 2.0), Vec3::new(9.0, 14.0, 2.0)]),
    hard_element(0xFFCB_F96E, [Vec3::new(-10.0, -6.0, 2.0), Vec3::new(9.0, -6.0, 2.0), Vec3::new(9.0, 14.0, 2.0)]),
];

/// `sHardTrisInit`: metal, turning hits away (`AC_HARD`).
pub const HARD_TRIS_INIT: ColliderInit = ColliderInit { col_type: COL_MATERIAL_METAL, at_flags: AT_NONE, ac_flags: AC_ON | AC_HARD | AC_TYPE_PLAYER, oc_flags1: OC1_NONE, oc_flags2: OC2_NONE, shape: COLSHAPE_TRIS };

/// `sAttackQuadInit`: the fire, `0x20000000` with `HIT_SPECIAL_EFFECT_FIRE`, 16.
pub const ATTACK_QUAD_INIT: ColliderQuadInit = ColliderQuadInit {
    base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_ON | AT_TYPE_ENEMY, ac_flags: AC_NONE, oc_flags1: OC1_NONE, oc_flags2: OC2_NONE, shape: COLSHAPE_QUAD },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0x2000_0000, hit_special_effect: HIT_SPECIAL_EFFECT_FIRE, damage: 0x10 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
        at_elem_flags: ATELEM_ON | ATELEM_SFX_NORMAL | ATELEM_UNK7,
        ac_elem_flags: ACELEM_NONE,
        oc_elem_flags: OCELEM_NONE,
    },
    quad: [Vec3::ZERO; 4],
};

/// `sDamageTable`.
pub static S_DAMAGE_TABLE: DamageTable = DamageTable {
    table: [
        dmg_entry(0, 0x1), // Deku nut
        dmg_entry(2, 0x0), // Deku stick
        dmg_entry(1, 0x0), // Slingshot
        dmg_entry(2, 0x0), // Explosive
        dmg_entry(0, 0x1), // Boomerang
        dmg_entry(1, 0x0), // Normal arrow
        dmg_entry(2, 0x0), // Hammer swing
        dmg_entry(0, 0x1), // Hookshot
        dmg_entry(1, 0x0), // Kokiri sword
        dmg_entry(2, 0x0), // Master sword
        dmg_entry(4, 0x0), // Giant's Knife
        dmg_entry(2, 0x0), // Fire arrow
        dmg_entry(4, 0xF), // Ice arrow
        dmg_entry(2, 0x0), // Light arrow
        dmg_entry(2, 0x0), // Unk arrow 1
        dmg_entry(2, 0x0), // Unk arrow 2
        dmg_entry(2, 0x0), // Unk arrow 3
        dmg_entry(0, 0x6), // Fire magic
        dmg_entry(3, 0xF), // Ice magic
        dmg_entry(0, 0x6), // Light magic
        dmg_entry(0, 0x0), // Shield
        dmg_entry(0, 0x0), // Mirror Ray
        dmg_entry(1, 0x0), // Kokiri spin
        dmg_entry(4, 0x0), // Giant spin
        dmg_entry(2, 0x0), // Master spin
        dmg_entry(2, 0x0), // Kokiri jump
        dmg_entry(8, 0x0), // Giant jump
        dmg_entry(4, 0x0), // Master jump
        dmg_entry(0, 0x0), // Unknown 1
        dmg_entry(0, 0x0), // Unblockable
        dmg_entry(4, 0x0), // Hammer jump
        dmg_entry(0, 0x0), // Unknown 2
    ],
};

/// `EnDodongo_UpdateQuad`'s corners on limb 2: `sp94` (quad 1), `sp88` (0), `sp7C` (3), `sp70`
/// (2); their x less the sweep (`sp70` not moved).
pub const QUAD_SP94: Vec3 = Vec3::new(-1000.0, -1500.0, 0.0);
pub const QUAD_SP88: Vec3 = Vec3::new(-1000.0, -200.0, 1500.0);
pub const QUAD_SP7C: Vec3 = Vec3::new(-1000.0, -200.0, -1500.0);
pub const QUAD_SP70: Vec3 = Vec3::ZERO;

/// `EnDodongo_PostLimbDraw`'s offsets: the tail's tip (limb 15), the head's triangles, the mouth
/// and the head (limb 7).
pub const TAIL_TIP_OFFSET: Vec3 = Vec3::new(3000.0, 0.0, 0.0);
pub const HARD_TRIS0_VTX_OFFSET: [Vec3; 3] = [Vec3::new(-300.0, -2500.0, 0.0), Vec3::new(-300.0, 1200.0, -2700.0), Vec3::new(3000.0, 1200.0, 0.0)];
pub const HARD_TRIS1_VTX_OFFSET: [Vec3; 3] = [Vec3::new(-300.0, -2500.0, 0.0), Vec3::new(-300.0, 1200.0, 2700.0), Vec3::new(3000.0, 1200.0, 0.0)];
pub const HARD_TRIS2_VTX_OFFSET: [Vec3; 3] = [Vec3::new(-600.0, 1200.0, -2800.0), Vec3::new(-600.0, 1200.0, 2800.0), Vec3::new(3000.0, 1200.0, 0.0)];
pub const MOUTH_OFFSET: Vec3 = Vec3::new(1800.0, 1200.0, 0.0);
pub const HEAD_OFFSET: Vec3 = Vec3::new(1500.0, 300.0, 0.0);
/// `legOffsets[1]` (the only one used: 0).
pub const LEG_OFFSET: Vec3 = Vec3::ZERO;

/// The limbs (1-based) `EnDodongo_PostLimbDraw` reads.
pub const LIMB_QUAD: usize = 2;
pub const LIMB_HEAD: usize = 7;
pub const LIMB_TAIL: usize = 15;
pub const LIMB_LEFT_FOOT: usize = 21;
pub const LIMB_RIGHT_FOOT: usize = 28;

/// The colliders' ids (`collider_mut`).
pub const COL_BODY: u8 = 0;
pub const COL_HARD: u8 = 1;
pub const COL_AT: u8 = 2;

/// The animations of `object_dodongo`.
#[derive(Default)]
struct Anims {
    die: Option<Anim>,
    damage: Option<Anim>,
    breathe_fire: Option<Anim>,
    after_breathe_fire: Option<Anim>,
    sweep_tail_right: Option<Anim>,
    sweep_tail_left: Option<Anim>,
    wait: Option<Anim>,
    walk: Option<Anim>,
}

// shared: DECR (`z_math.h`)
/// `DECR(x)`: down to 0, the new value.
fn decr(x: &mut i16) -> i16 {
    if *x != 0 {
        *x -= 1;
    }
    *x
}

/// `Math_Vec3f_DistXZ`.
fn dist_xz(a: Vec3, b: Vec3) -> f32 {
    let (dx, dz) = (b.x - a.x, b.z - a.z);
    (dx * dx + dz * dz).sqrt()
}

/// `EnDodongo_ShiftVecRadial`: `vec` moved `radius` along `yaw`.
pub fn shift_vec_radial(yaw: i16, radius: f32, vec: &mut Vec3) {
    vec.x += sin_s(yaw) * radius;
    vec.z += cos_s(yaw) * radius;
}

/// `EnDodongo_OverrideLimbDraw`'s matrix: limbs 15 and 16 scaled by `bodyScale` before their own
/// transform.
pub fn limb_pre(limb: usize, body_scale: Vec3) -> Mat4 {
    if limb == 15 || limb == 16 { Mat4::from_scale(body_scale) } else { Mat4::IDENTITY }
}

pub struct EnDodongo {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    skeleton: Option<Arc<Skeleton>>,
    anims: Anims,
    /// `actionState`.
    pub action_state: ActionState,
    pub action: Action,
    /// `timer`: idling, walking, swallowing, sweeping, dying.
    pub timer: i16,
    pub retreat_timer: i16,
    /// `tailSwipeSpeed`: the sweep's turn a frame.
    pub tail_swipe_speed: i16,
    pub ice_timer: i16,
    /// `rightFootStep`: the next step is the right foot's.
    pub right_foot_step: bool,
    pub left_foot_pos: Vec3,
    pub right_foot_pos: Vec3,
    /// `mouthPos`, `headPos`: from the last draw (limb 7).
    pub mouth_pos: Vec3,
    pub head_pos: Vec3,
    /// `bodyScale`: limbs 15 and 16's swell.
    pub body_scale: Vec3,
    pub ice_pos: [Vec3; 9],
    pub bomb_smoke_prim_color: [u8; 4],
    pub bomb_smoke_env_color: [u8; 4],
    /// `damageReaction`: the last hit's.
    pub damage_reaction: u8,
    /// `blureIdx`: the tail's trail (`TOTAL_EFFECT_COUNT`, the effects' count, with no slot).
    pub blure_idx: usize,
    pub collider_at: ColliderQuad,
    pub collider_hard: ColliderTris,
    pub body_collider: ColliderJntSph,
}

impl EnDodongo {
    /// `EnDodongo_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // ATTENTION_RANGE_3.
        actor.target_mode = 3;
        // sInitChain: naviEnemyId NAVI_ENEMY_DODONGO, gravity -1, lockOnArrowOffset 2800.
        actor.navi_enemy_id = NAVI_ENEMY_DODONGO;
        actor.gravity = -1.0;
        actor.target_arrow_offset = 2800.0;
        actor.shape_init(0.0, oot_game::shadow::ShadowDraw::Circle, 48.0);
        actor.scale = Vec3::splat(0.01875);
        let mut anims = Anims::default();
        let mut skeleton = None;
        if let Some(a) = play.assets.clone() {
            let anim = |name: &str| a.animation(OBJECT, name).map_err(|e| log::error!("En_Dodongo: {e:#}")).ok();
            skeleton = a.skeleton(OBJECT, SKEL).map_err(|e| log::error!("En_Dodongo: {e:#}")).ok();
            anims = Anims {
                die: anim(ANIM_DIE),
                damage: anim(ANIM_DAMAGE),
                breathe_fire: anim(ANIM_BREATHE_FIRE),
                after_breathe_fire: anim(ANIM_AFTER_BREATHE_FIRE),
                sweep_tail_right: anim(ANIM_SWEEP_TAIL_RIGHT),
                sweep_tail_left: anim(ANIM_SWEEP_TAIL_LEFT),
                wait: anim(ANIM_WAIT),
                walk: anim(ANIM_WALK),
            };
        }
        // SkelAnime_Init(.., &gDodongoSkel, &gDodongoWaitAnim, .., 31): 30 limbs.
        let limbs = skeleton.as_ref().map(|s| s.limbs.len()).unwrap_or(30);
        let skel = SkelAnimeStd::init_flex(limbs, anims.wait.clone());
        actor.col_chk_info.health = 4;
        actor.col_chk_info.mass = MASS_HEAVY;
        actor.col_chk_info.damage_table = Some(&S_DAMAGE_TABLE);
        // EFFECT_BLURE1: white, p1 from 255 to 0, p2 from 64 to 0, elemDuration 8, calcMode 2.
        let blure_idx = play.effect_add(EffectInit::Blure(EffectBlureInit::One(EffectBlureInit1 {
            p1_start_color: [255, 255, 255, 255],
            p2_start_color: [255, 255, 255, 64],
            p1_end_color: [255, 255, 255, 0],
            p2_end_color: [255, 255, 255, 0],
            elem_duration: 8,
            unk_flag: 0,
            calc_mode: 2,
        })));
        actor.update_bg_check_info(&play.col, 75.0, 60.0, 70.0, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2 | UPDBGCHECKINFO_FLAG_3 | UPDBGCHECKINFO_FLAG_4);
        let mut this = EnDodongo {
            actor,
            skel,
            skeleton,
            anims,
            action_state: ActionState::Idle,
            action: Action::Idle,
            timer: 0,
            retreat_timer: 0,
            tail_swipe_speed: 0,
            ice_timer: 0,
            right_foot_step: false,
            left_foot_pos: Vec3::ZERO,
            right_foot_pos: Vec3::ZERO,
            mouth_pos: Vec3::ZERO,
            head_pos: Vec3::ZERO,
            body_scale: Vec3::ONE,
            ice_pos: [Vec3::ZERO; 9],
            // prim.r = prim.g = env.r = 255; prim.a = env.a = 200; env.g = 10 (the rest 0).
            bomb_smoke_prim_color: [255, 255, 0, 200],
            bomb_smoke_env_color: [255, 10, 0, 200],
            damage_reaction: 0,
            blure_idx,
            collider_at: ColliderQuad::new(&ATTACK_QUAD_INIT),
            collider_hard: ColliderTris::new(&HARD_TRIS_INIT, &HARD_ELEMENTS),
            body_collider: ColliderJntSph::new(&BODY_JNT_SPH_INIT, &BODY_ELEMENTS),
        };
        this.setup_idle(play);
        Box::new(this)
    }

    /// `Animation_Change(&skelAnime, anim, speed, start, end (the last frame if `None`), mode,
    /// morph)`.
    fn change(&mut self, anim: Option<Anim>, speed: f32, start: f32, end: Option<f32>, mode: u8, morph: f32) {
        if let Some(a) = anim {
            let end = end.unwrap_or_else(|| a.last_frame());
            self.skel.change(a, speed, start, end, mode, morph);
        }
    }

    /// `EnDodongo_SpawnBombSmoke`: three puffs (`func_8002836C`, 100 big, 25 growing, 20 frames)
    /// from its head, the colours darkening and the env's red fading 15 a call, blown out to the
    /// sides (the first within 0x2000 of its facing either way, the others 6 to either side).
    fn spawn_bomb_smoke(&mut self, play: &mut PlayState) {
        let head = self.head_pos;
        let rot_y = self.actor.shape_rot.y;
        let (mut prim, mut env) = (self.bomb_smoke_prim_color, self.bomb_smoke_env_color);
        play.with_ss(|ss| {
            let mut velocity = Vec3::ZERO;
            let mut accel = Vec3::new(0.0, 0.3, 0.0);
            let mut pos = head;
            let mut rand_angle = ss.rand.centered_float(16384.0) as i32 as i16;
            let mut rand_cos = cos_s(rot_y.wrapping_add(rand_angle));
            let mut rand_sin = sin_s(rot_y.wrapping_add(rand_angle));
            if prim[0] > 30 {
                prim[0] = prim[0].wrapping_sub(16);
                prim[1] = prim[1].wrapping_sub(16);
            }
            if prim[2] < 30 {
                prim[2] = prim[2].wrapping_add(5);
                prim[3] = prim[3].wrapping_add(8);
                env[3] = env[3].wrapping_add(8);
            }
            if env[0] != 0 {
                env[0] = env[0].wrapping_sub(15);
            }
            if env[1] != 0 {
                env[1] -= 1;
            }
            velocity.x = rand_sin * 3.5;
            velocity.y = env[0] as f32 * 0.02;
            velocity.z = rand_cos * 3.5;
            accel.x = ((ss.rand.zero_one() * 0.1) + 0.15) * -rand_sin;
            accel.z = ((ss.rand.zero_one() * 0.1) + 0.15) * -rand_cos;
            ss.func_8002836c(pos, velocity, accel, prim, env, 100, 25, 20);

            rand_angle = (ss.rand.zero_one() * 8192.0) as i32 as i16;
            rand_cos = cos_s(rot_y.wrapping_add(rand_angle));
            rand_sin = sin_s(rot_y.wrapping_add(rand_angle));
            pos.x -= rand_cos * 6.0;
            pos.z += rand_sin * 6.0;
            velocity.x = -rand_cos * 3.5;
            velocity.y = env[0] as f32 * 0.02;
            velocity.z = rand_sin * 3.5;
            accel.x = ((ss.rand.zero_one() * 0.1) + 0.15) * rand_cos;
            accel.z = ((ss.rand.zero_one() * 0.1) + 0.15) * -rand_sin;
            ss.func_8002836c(pos, velocity, accel, prim, env, 100, 25, 20);

            rand_angle = (ss.rand.zero_one() * 8192.0) as i32 as i16;
            rand_cos = cos_s(rot_y.wrapping_add(rand_angle));
            rand_sin = sin_s(rot_y.wrapping_add(rand_angle));
            pos.x = head.x + (rand_cos * 6.0);
            pos.z = head.z - (rand_sin * 6.0);
            velocity.x = rand_cos * 3.5;
            velocity.y = env[0] as f32 * 0.02;
            velocity.z = -rand_sin * 3.5;
            accel.x = ((ss.rand.zero_one() * 0.1) + 0.15) * -rand_cos;
            accel.z = ((ss.rand.zero_one() * 0.1) + 0.15) * rand_sin;
            ss.func_8002836c(pos, velocity, accel, prim, env, 100, 25, 20);
        });
        self.bomb_smoke_prim_color = prim;
        self.bomb_smoke_env_color = env;
    }

    /// `EnDodongo_SetupIdle`: the wait looped (morph 4), stopped, `Rand_S16Offset(30, 50)` frames.
    fn setup_idle(&mut self, play: &mut PlayState) {
        self.change(self.anims.wait.clone(), 1.0, 0.0, None, ANIMMODE_LOOP, -4.0);
        self.actor.speed_xz = 0.0;
        self.timer = play.rand.s16_offset(30, 50);
        self.action_state = ActionState::Idle;
        self.action = Action::Idle;
    }

    /// `EnDodongo_SetupWalk`: the walk looped, still (speed 0, morph 4), at 1.5,
    /// `Rand_S16Offset(50, 70)` frames, the right foot next.
    fn setup_walk(&mut self, play: &mut PlayState) {
        self.change(self.anims.walk.clone(), 0.0, 0.0, None, ANIMMODE_LOOP, -4.0);
        self.actor.speed_xz = 1.5;
        self.timer = play.rand.s16_offset(50, 70);
        self.right_foot_step = true;
        self.action_state = ActionState::Walk;
        self.action = Action::Walk;
    }

    /// `EnDodongo_SetupBreatheFire`: the fire once (morph 4), stopped.
    fn setup_breathe_fire(&mut self) {
        self.change(self.anims.breathe_fire.clone(), 1.0, 0.0, None, ANIMMODE_ONCE, -4.0);
        self.action_state = ActionState::BreatheFire;
        self.actor.speed_xz = 0.0;
        self.action = Action::BreatheFire;
    }

    /// `EnDodongo_SetupEndBreatheFire`: `gDodongoAfterBreatheFireAnim` once, stopped.
    fn setup_end_breathe_fire(&mut self) {
        if let Some(a) = self.anims.after_breathe_fire.clone() {
            self.skel.play_once(a);
        }
        self.action_state = ActionState::EndBreatheFire;
        self.actor.speed_xz = 0.0;
        self.action = Action::EndBreatheFire;
    }

    /// `EnDodongo_SetupSwallowBomb`: the fire backwards from 35 to 0 (morph 4), 25 frames, stopped.
    fn setup_swallow_bomb(&mut self) {
        self.change(self.anims.breathe_fire.clone(), -1.0, 35.0, Some(0.0), ANIMMODE_ONCE, -4.0);
        self.action_state = ActionState::SwallowBomb;
        self.timer = 25;
        self.actor.speed_xz = 0.0;
        self.action = Action::SwallowBomb;
    }

    /// `EnDodongo_SetupStunned`: the fire held at 25 (morph 4), stopped; ice (reaction 0xF) 36
    /// frames; `NA_SE_EN_GOMA_JR_FREEZE`.
    fn setup_stunned(&mut self, play: &mut PlayState) {
        self.change(self.anims.breathe_fire.clone(), 0.0, 25.0, Some(0.0), ANIMMODE_ONCE, -4.0);
        self.action_state = ActionState::Stunned;
        self.actor.speed_xz = 0.0;
        if self.damage_reaction == 0xF {
            self.ice_timer = 36;
        }
        audio_play_actor_sfx2(play, NA_SE_EN_GOMA_JR_FREEZE);
        self.action = Action::Stunned;
    }

    /// `EnDodongo_Idle`: its frames out (`DECR`) at the loop's frame 0, walking.
    fn idle(&mut self, play: &mut PlayState) {
        self.skel.update();
        if decr(&mut self.timer) == 0 && self.skel.on_frame(0.0) {
            self.setup_walk(play);
        }
    }

    /// `EnDodongo_EndBreatheFire`: the animation done, idle for `Rand_S16Offset(10, 20)`.
    fn end_breathe_fire(&mut self, play: &mut PlayState) {
        if self.skel.update() {
            self.setup_idle(play);
            self.timer = play.rand.s16_offset(10, 20);
        }
    }

    /// `EnDodongo_BreatheFire`: `NA_SE_EN_DODO_J_CRY` at 24; frames 29 to 43 the fire's sound
    /// and a puff (`EffectSsDFire_SpawnFixedScale`: 35 up, 30 ahead, pushed 2.5 a frame, alpha
    /// `255 - 10 × (frame - 29)`, fading with `frame - 26` frames left); 2 to 20 the breath's
    /// sound; the animation done, `EnDodongo_SetupEndBreatheFire`.
    fn breathe_fire(&mut self, play: &mut PlayState) {
        let velocity = Vec3::ZERO;
        let mut accel = Vec3::ZERO;
        let cur = self.skel.cur_frame;
        if cur as i32 == 24 {
            audio_play_actor_sfx2(play, NA_SE_EN_DODO_J_CRY);
        }
        if (29.0..=43.0).contains(&cur) {
            audio_play_actor_sfx2(play, NA_SE_EN_DODO_J_FIRE - SFX_FLAG);
            let fire_frame = (cur - 29.0) as i32 as i16;
            let mut pos = self.actor.world_pos;
            pos.y += 35.0;
            shift_vec_radial(self.actor.world_rot.y, 30.0, &mut pos);
            shift_vec_radial(self.actor.world_rot.y, 2.5, &mut accel);
            let alpha = 255 - (fire_frame * 10);
            play.with_ss(|ss| ss.d_fire_spawn_fixed_scale(pos, velocity, accel, alpha, fire_frame + 3));
        } else if (2.0..=20.0).contains(&cur) {
            audio_play_actor_sfx2(play, NA_SE_EN_DODO_J_BREATH - SFX_FLAG);
        }
        if self.skel.update() {
            self.setup_end_breathe_fire();
        }
    }

    /// The explosive it holds: its `child` (an `En_Bom`), else its `parent` (any other).
    fn held_explosive(&self) -> Option<(ActorHandle, bool)> {
        self.actor.child.map(|h| (h, true)).or(self.actor.parent.map(|h| (h, false)))
    }

    /// `EnDodongo_SwallowBomb`: the explosive held at its mouth with its fuse held (`timer` up
    /// one); at frame 28 `NA_SE_EN_DODO_J_EAT` and the explosive gone; at 24, held there (one
    /// frame back) while `timer` counts down, at 10 the blast inside (eleven puffs from its
    /// body's first sphere, `NA_SE_IT_BOMB_EXPLOSION`, red 8 frames); below 28 smoke: black
    /// (`EnDodongo_SpawnBombSmoke`) below 26 from 10 left, else white; its body swelling with
    /// the red; `timer` out, dead.
    fn swallow_bomb(&mut self, play: &mut PlayState) {
        let smoke_vel = Vec3::ZERO;
        let smoke_accel = Vec3::new(0.0, 0.6, 0.0);
        let white = [255, 255, 255, 255];
        let mouth = self.mouth_pos;
        match self.held_explosive() {
            Some((h, true)) => {
                if let Some(bomb) = play.actors.downcast_mut::<EnBom>(h) {
                    bomb.actor.world_pos = mouth;
                    bomb.timer = bomb.timer.wrapping_add(1);
                }
            }
            Some((h, false)) => {
                if let Some(bombf) = play.actors.downcast_mut::<EnBombf>(h) {
                    bombf.actor.world_pos = mouth;
                    bombf.timer = bombf.timer.wrapping_add(1);
                } else if let Some(a) = play.actors.actor_mut(h) {
                    a.world_pos = mouth;
                    // @bug (game): a Bombchu's `timer` (0x1F8) is past its end: the C corrupts
                    // the next arena node. Not ported (no Bombchu is).
                    log::warn!("En_Dodongo: ((EnBombf*)parent)->timer++ on a non-En_Bombf explosive (id {:#06x}): not ported", a.id);
                }
            }
            None => {}
        }
        let cur = self.skel.cur_frame as i32;
        if cur == 28 {
            audio_play_actor_sfx2(play, NA_SE_EN_DODO_J_EAT);
            if let Some(h) = self.actor.child {
                if let Some(a) = play.actors.actor_mut(h) {
                    a.kill();
                }
                self.actor.child = None;
            } else if let Some(h) = self.actor.parent {
                if let Some(a) = play.actors.actor_mut(h) {
                    a.kill();
                }
                self.actor.parent = None;
            }
        } else if cur == 24 {
            self.timer -= 1;
            if self.timer != 0 {
                self.skel.cur_frame += 1.0;
                if self.timer == 10 {
                    let c = self.body_collider.elements[0].dim.world_sphere.center;
                    let center = Vec3::new(c[0] as f32, c[1] as f32, c[2] as f32);
                    let (prim, env) = (self.bomb_smoke_prim_color, self.bomb_smoke_env_color);
                    play.with_ss(|ss| {
                        for _ in (0..=10).rev() {
                            let v = Vec3::new(ss.rand.centered_float(10.0), ss.rand.centered_float(10.0), ss.rand.centered_float(10.0));
                            let a = v * -0.1;
                            ss.func_8002836c(center + v, v, a, prim, env, 400, 10, 10);
                        }
                    });
                    audio_play_actor_sfx2(play, NA_SE_IT_BOMB_EXPLOSION);
                    self.actor.set_color_filter(COLORFILTER_COLORFLAG_RED, 120, COLORFILTER_BUFFLAG_OPA, 8);
                }
            }
        }
        if (self.skel.cur_frame as i32) < 28 {
            if (self.skel.cur_frame as i32) < 26 && self.timer <= 10 {
                self.spawn_bomb_smoke(play);
            } else {
                let head = self.head_pos;
                let rot_y = self.actor.shape_rot.y;
                play.with_ss(|ss| {
                    let mut pos = head;
                    ss.func_8002829c(pos, smoke_vel, smoke_accel, white, white, 50, 5);
                    pos.x -= cos_s(rot_y) * 6.0;
                    pos.z += sin_s(rot_y) * 6.0;
                    ss.func_8002829c(pos, smoke_vel, smoke_accel, white, white, 50, 5);
                    pos.x = head.x + (cos_s(rot_y) * 6.0);
                    pos.z = head.z - (sin_s(rot_y) * 6.0);
                    ss.func_8002829c(pos, smoke_vel, smoke_accel, white, white, 50, 5);
                });
            }
        }
        let s = sin_s((self.actor.color_filter_timer as i32 * 0x1000) as i16);
        self.body_scale.y = (s * 0.5) + 1.0;
        self.body_scale.z = self.body_scale.y;
        self.body_scale.x = s + 1.0;
        self.skel.update();
        if self.timer == 0 {
            self.setup_death(play);
        }
    }

    /// `EnDodongo_Walk`.
    fn walk(&mut self, play: &mut PlayState) {
        let player_pos = play.player.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos).unwrap_or_default();
        // (s16)ABS((s16)(yawTowardsPlayer - shape.rot.y)): -0x8000 stays.
        let yaw_diff = self.actor.yaw_towards_player.wrapping_sub(self.actor.shape_rot.y).wrapping_abs();
        smooth_step_to_f(&mut self.actor.speed_xz, 1.5, 0.1, 1.0, 0.0);
        let mut playback_speed = self.actor.speed_xz * 0.75;
        if self.actor.speed_xz >= 0.0 {
            if playback_speed > 3.0 / 2.0 {
                playback_speed = 3.0 / 2.0;
            }
        } else if playback_speed < -3.0 / 2.0 {
            playback_speed = -3.0 / 2.0;
        }
        self.skel.play_speed = playback_speed;
        self.skel.update();
        if (self.skel.cur_frame as i32) < 21 {
            if !self.right_foot_step {
                audio_play_actor_sfx2(play, NA_SE_EN_DODO_J_WALK);
                let a = self.actor.clone();
                actor_spawn_floor_dust_ring(play, &a, self.left_foot_pos, 10.0, 3, 2.0, 200, 15, false);
                self.right_foot_step = true;
            }
        } else if self.right_foot_step {
            audio_play_actor_sfx2(play, NA_SE_EN_DODO_J_WALK);
            let a = self.actor.clone();
            actor_spawn_floor_dust_ring(play, &a, self.right_foot_pos, 10.0, 3, 2.0, 200, 15, false);
            self.right_foot_step = false;
        }
        if dist_xz(self.actor.home_pos, player_pos) < 400.0 {
            smooth_step_to_s(&mut self.actor.world_rot.y, self.actor.yaw_towards_player, 1, 0x1F4, 0);
            self.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED;
            if self.actor.xz_dist_to_player < 100.0 && yaw_diff < 0x1388 && self.actor.y_dist_to_player < 60.0 {
                self.setup_breathe_fire();
            }
        } else {
            self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
            if dist_xz(self.actor.world_pos, self.actor.home_pos) > 150.0 || self.retreat_timer != 0 {
                let yaw_to_home = vec3f_yaw(self.actor.world_pos, self.actor.home_pos);
                smooth_step_to_s(&mut self.actor.world_rot.y, yaw_to_home, 1, 0x1F4, 0);
            }
            if self.retreat_timer != 0 {
                self.retreat_timer -= 1;
            }
            self.timer -= 1;
            if self.timer == 0 {
                if play.rand.zero_one() > 0.7 {
                    self.timer = play.rand.s16_offset(50, 70);
                    self.retreat_timer = play.rand.s16_offset(15, 40);
                } else {
                    self.setup_idle(play);
                }
            }
        }
        self.actor.shape_rot.y = self.actor.world_rot.y;
    }

    /// `EnDodongo_SetupSweepTail`: `gDodongoDamageAnim` once (morph 4), `NA_SE_EN_DODO_J_DAMAGE`,
    /// `timer` 0, stopped.
    fn setup_sweep_tail(&mut self, play: &mut PlayState) {
        self.change(self.anims.damage.clone(), 1.0, 0.0, None, ANIMMODE_ONCE, -4.0);
        audio_play_actor_sfx2(play, NA_SE_EN_DODO_J_DAMAGE);
        self.action_state = ActionState::SweepTail;
        self.timer = 0;
        self.actor.speed_xz = 0.0;
        self.action = Action::SweepTail;
    }

    /// The tail's AT (`bodyColliderElements[1]` and `[2]`) and the collider's flags, as the sweep
    /// writes them.
    fn set_tail_at(&mut self, flags: u8, dmg_flags: u32, damage: u8) {
        self.body_collider.base.at_flags = flags;
        for i in [1, 2] {
            let e = &mut self.body_collider.elements[i].info;
            e.at_elem_flags = flags;
            e.at_dmg_info.dmg_flags = dmg_flags;
            e.at_dmg_info.damage = damage;
        }
    }

    /// `EnDodongo_SweepTail`: an animation done: with a sweep behind it or Link within 0x4000 of
    /// its facing, the tail's AT off and the fire (`timer` `Rand_S16Offset(5, 10)`); else a sweep
    /// his way (`tailSwipeSpeed` `(0xFFFF - |yaw|) / 15`, negative with him to the left: the left
    /// sweep; the right else) at 2, `NA_SE_EN_DODO_J_TAIL`, 18 frames, the tail's AT on (written
    /// `AT_ON | AT_TYPE_ENEMY` to the collider and, as `ATELEM_ON | ATELEM_SFX_WOOD`, the two
    /// spheres; `DMG_DEFAULT`, 8). Sweeping (`timer` above 1): turning, dust under the tail's
    /// spheres, `NA_SE_PL_BODY_HIT` on Link when it hit him, the AT registered.
    fn sweep_tail(&mut self, play: &mut PlayState) {
        let yaw_diff1 = self.actor.yaw_towards_player.wrapping_sub(self.actor.shape_rot.y);
        if self.skel.update() {
            if self.timer != 0 || (yaw_diff1 as i32).abs() < 0x4000 {
                self.set_tail_at(AT_NONE, 0, 0);
                self.setup_breathe_fire();
                self.timer = play.rand.s16_offset(5, 10);
            } else {
                let yaw_diff2 = self.actor.yaw_towards_player.wrapping_sub(self.actor.shape_rot.y);
                self.tail_swipe_speed = ((0xFFFF - (yaw_diff2 as i32).abs()) / 0xF) as i16;
                let animation = if self.actor.yaw_towards_player.wrapping_sub(self.actor.shape_rot.y) >= 0 {
                    self.tail_swipe_speed = -self.tail_swipe_speed;
                    self.anims.sweep_tail_left.clone()
                } else {
                    self.anims.sweep_tail_right.clone()
                };
                audio_play_actor_sfx2(play, NA_SE_EN_DODO_J_TAIL);
                // Animation_PlayOnceSetSpeed(&skelAnime, animation, 2.0f).
                self.change(animation, 2.0, 0.0, None, ANIMMODE_ONCE, 0.0);
                self.timer = 18;
                // atFlags = atElemFlags = AT_ON | AT_TYPE_ENEMY: also ATELEM_ON | ATELEM_SFX_WOOD.
                self.set_tail_at(AT_ON | AT_TYPE_ENEMY, DMG_DEFAULT, 8);
            }
        } else if self.timer > 1 {
            self.timer -= 1;
            self.actor.world_rot.y = self.actor.world_rot.y.wrapping_add(self.tail_swipe_speed);
            self.actor.shape_rot.y = self.actor.world_rot.y;
            let a = self.actor.clone();
            for i in [1, 2] {
                let c = self.body_collider.elements[i].dim.world_sphere.center;
                let tail_pos = Vec3::new(c[0] as f32, c[1] as f32, c[2] as f32);
                actor_spawn_floor_dust_ring(play, &a, tail_pos, 5.0, 2, 2.0, 100, 15, false);
            }
            if self.body_collider.base.at_flags & AT_HIT != 0
                && let Some(p) = play.player
                && self.body_collider.base.at == Some(p)
            {
                audio_play_actor_sfx_at(play, p, NA_SE_PL_BODY_HIT);
            }
            play.collision_check_set_at(&self.actor, COL_BODY, &mut self.body_collider);
        }
    }

    /// `EnDodongo_SetupDeath`: `gDodongoDieAnim` once (morph 8), `timer` 0,
    /// `NA_SE_EN_DODO_J_DEAD`, untargetable, stopped.
    fn setup_death(&mut self, play: &mut PlayState) {
        self.change(self.anims.die.clone(), 1.0, 0.0, None, ANIMMODE_ONCE, -8.0);
        self.timer = 0;
        audio_play_actor_sfx2(play, NA_SE_EN_DODO_J_DEAD);
        self.action_state = ActionState::Death;
        self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        self.actor.speed_xz = 0.0;
        self.action = Action::Death;
    }

    /// `EnDodongo_Death`: to frame 35 the smoke (params 0); then red flashes of 4;
    /// `NA_SE_EN_RIZA_DOWN` at 52; the animation done, an `En_Bom` (`BOMB_BODY`, rot z 6) with its
    /// fuse at 0 where it lies, and 8 frames (7 after this one) to the drop (table 4) and its end.
    fn death(&mut self, play: &mut PlayState) {
        if self.skel.cur_frame < 35.0 {
            if PARAMS.get(self.actor.params) == EN_DODONGO_SMOKE_DEATH {
                self.spawn_bomb_smoke(play);
            }
        } else if self.actor.color_filter_timer == 0 {
            self.actor.set_color_filter(COLORFILTER_COLORFLAG_RED, 120, COLORFILTER_BUFFLAG_OPA, 4);
        }
        if self.skel.update() {
            if self.timer == 0 {
                let p = self.actor.world_pos;
                match play.actor_spawn(ACTOR_EN_BOM, p, [0, 0, 6], BOMB_BODY) {
                    Ok(h) => {
                        if let Some(bomb) = play.actors.downcast_mut::<EnBom>(h) {
                            bomb.timer = 0;
                            self.timer = 8;
                        }
                    }
                    Err(e) => log::debug!("En_Dodongo: no bomb: {e:?}"),
                }
            }
        } else if self.skel.cur_frame as i32 == 52 {
            audio_play_actor_sfx2(play, NA_SE_EN_RIZA_DOWN);
        }
        if self.timer != 0 {
            self.timer -= 1;
            if self.timer == 0 {
                let pos = self.actor.world_pos;
                crate::en_item00::item_drop_collectible_random(play, Some(&self.actor), pos, DROP_TABLE_4);
                self.actor.kill();
            }
        }
    }

    /// `EnDodongo_Stunned`: its colour out, dead at no health, else idle.
    fn stunned(&mut self, play: &mut PlayState) {
        self.skel.update();
        if self.actor.color_filter_timer == 0 {
            if self.actor.col_chk_info.health == 0 {
                self.setup_death(play);
            } else {
                self.setup_idle(play);
            }
        }
    }

    /// `EnDodongo_CollisionCheck`: a hit turned away by the head's metal cancels the body's; a body
    /// hit (not swallowing, dying or sweeping) with any reaction but 0xE: reaction 1 or 0xF, not
    /// stunned already, blue 80 frames, the damage, stunned; any other, red 8 frames, the damage,
    /// dead at 0, else the sweep.
    fn collision_check(&mut self, play: &mut PlayState) {
        if self.collider_hard.base.ac_flags & AC_BOUNCED != 0 {
            self.collider_hard.base.ac_flags &= !AC_BOUNCED;
            self.body_collider.base.ac_flags &= !AC_HIT;
        } else if self.body_collider.base.ac_flags & AC_HIT != 0 && self.action_state > ActionState::Death {
            self.body_collider.base.ac_flags &= !AC_HIT;
            self.actor.set_drop_flag_jnt_sph(&self.body_collider, false);
            let reaction = self.actor.col_chk_info.damage_reaction;
            if reaction != 0xE {
                self.damage_reaction = reaction;
                if reaction == 1 || reaction == 0xF {
                    if self.action_state != ActionState::Stunned {
                        self.actor.set_color_filter(COLORFILTER_COLORFLAG_BLUE, 120, COLORFILTER_BUFFLAG_OPA, 80);
                        self.actor.apply_damage();
                        self.setup_stunned(play);
                    }
                } else {
                    self.actor.set_color_filter(COLORFILTER_COLORFLAG_RED, 120, COLORFILTER_BUFFLAG_OPA, 8);
                    if self.actor.apply_damage() == 0 {
                        self.setup_death(play);
                    } else {
                        self.setup_sweep_tail(play);
                    }
                }
            }
        }
    }

    /// `EnDodongo_UpdateQuad`: the fire's quad on limb 2's matrix `m`, its x less
    /// `sinf((curFrame - 28) × 0.08) × 5500`.
    fn update_quad(&mut self, m: Mat4) {
        let x_mod = ((self.skel.cur_frame - 28.0) * 0.08).sin() * 5500.0;
        let mut sp94 = QUAD_SP94;
        let mut sp88 = QUAD_SP88;
        let mut sp7c = QUAD_SP7C;
        sp7c.x -= x_mod;
        sp94.x -= x_mod;
        sp88.x -= x_mod;
        let b = m.transform_point3(sp94);
        let a = m.transform_point3(sp88);
        let d = m.transform_point3(sp7c);
        let c = m.transform_point3(QUAD_SP70);
        self.collider_at.set_vertices(a, b, c, d);
    }

    /// The fire's quad is placed and registered: breathing fire, past frame 29 and before 43.
    fn fire_quad_on(&self) -> bool {
        self.action_state == ActionState::BreatheFire && 29.0 < self.skel.cur_frame && self.skel.cur_frame < 43.0
    }

    /// `EnDodongo_PostLimbDraw` at limb `limb` (1-based) with its matrix `m` (the model's in).
    fn post_limb_draw(&mut self, play: &mut PlayState, limb: usize, m: Mat4) {
        self.body_collider.update_spheres(limb as u8, &m);
        let mut base_offset = Vec3::ZERO;
        match limb {
            LIMB_QUAD => {
                if self.fire_quad_on() {
                    self.update_quad(m);
                }
            }
            LIMB_HEAD => {
                let t0 = HARD_TRIS0_VTX_OFFSET.map(|v| m.transform_point3(v));
                let t1 = HARD_TRIS1_VTX_OFFSET.map(|v| m.transform_point3(v));
                let t2 = HARD_TRIS2_VTX_OFFSET.map(|v| m.transform_point3(v));
                self.collider_hard.set_vertices(0, t0[0], t0[1], t0[2]);
                self.collider_hard.set_vertices(1, t1[0], t1[1], t1[2]);
                self.collider_hard.set_vertices(2, t2[0], t2[1], t2[2]);
                self.mouth_pos = m.transform_point3(MOUTH_OFFSET);
                self.head_pos = m.transform_point3(HEAD_OFFSET);
            }
            LIMB_TAIL => {
                if self.action_state == ActionState::SweepTail && self.timer >= 2 {
                    let tail_tip = m.transform_point3(TAIL_TIP_OFFSET);
                    let tail_base = m.transform_point3(base_offset);
                    if let Some(b) = play.effect_blure_mut(self.blure_idx) {
                        b.add_vertex(tail_tip, tail_base);
                    }
                } else if self.action_state == ActionState::SweepTail && self.timer != 0 {
                    if let Some(b) = play.effect_blure_mut(self.blure_idx) {
                        b.add_space();
                    }
                }
            }
            LIMB_LEFT_FOOT => self.left_foot_pos = m.transform_point3(LEG_OFFSET),
            LIMB_RIGHT_FOOT => self.right_foot_pos = m.transform_point3(LEG_OFFSET),
            _ => {}
        }
        if self.ice_timer != 0 {
            let i = match limb {
                7 => {
                    base_offset.x = 1200.0;
                    Some(0)
                }
                13 => Some(1),
                14 => Some(2),
                15 => Some(3),
                16 => Some(4),
                22 => Some(5),
                29 => Some(6),
                21 => Some(7),
                28 => Some(8),
                _ => None,
            };
            if let Some(i) = i {
                self.ice_pos[i] = m.transform_point3(base_offset);
            }
        }
    }
}

impl ActorImpl for EnDodongo {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnDodongo_Update`: the hits; with any reaction but 0xE the action,
    /// `Actor_MoveXZGravity`, the floor and walls (75 up, 60 across, 70 to the ceiling) and
    /// `NA_SE_EN_RIZA_DOWN` on landing; the body pushing, the head's metal hit but dying, the body
    /// hit but swallowing, dying or sweeping; idle or later, a bomb at its mouth swallowed; the
    /// fire's quad (past 29, before 43); the focus 30 behind its facing, 20 up.
    fn update(&mut self, play: &mut PlayState) {
        self.collision_check(play);
        if self.actor.col_chk_info.damage_reaction != 0xE {
            match self.action {
                Action::Idle => self.idle(play),
                Action::EndBreatheFire => self.end_breathe_fire(play),
                Action::BreatheFire => self.breathe_fire(play),
                Action::SwallowBomb => self.swallow_bomb(play),
                Action::Walk => self.walk(play),
                Action::Stunned => self.stunned(play),
                Action::Death => self.death(play),
                Action::SweepTail => self.sweep_tail(play),
            }
            self.actor.move_forward();
            self.actor.update_bg_check_info(&play.col, 75.0, 60.0, 70.0, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2 | UPDBGCHECKINFO_FLAG_3 | UPDBGCHECKINFO_FLAG_4);
            if self.actor.bg_check_flags & BGCHECKFLAG_GROUND_TOUCH != 0 {
                audio_play_actor_sfx2(play, NA_SE_EN_RIZA_DOWN);
            }
        }
        play.collision_check_set_oc(&self.actor, COL_BODY, &mut self.body_collider);
        if self.action_state != ActionState::Death {
            play.collision_check_set_ac(&self.actor, COL_HARD, &mut self.collider_hard);
        }
        if self.action_state > ActionState::Death {
            play.collision_check_set_ac(&self.actor, COL_BODY, &mut self.body_collider);
        }
        if self.action_state >= ActionState::Idle && self.ate_bomb(play) {
            self.setup_swallow_bomb();
        }
        if self.action_state == ActionState::BreatheFire && 29.0 < self.skel.cur_frame && self.skel.cur_frame < 43.0 {
            play.collision_check_set_at(&self.actor, COL_AT, &mut self.collider_at);
        }
        let rot_y = self.actor.shape_rot.y;
        self.actor.focus_pos.x = self.actor.world_pos.x + sin_s(rot_y) * -30.0;
        self.actor.focus_pos.y = self.actor.world_pos.y + 20.0;
        self.actor.focus_pos.z = self.actor.world_pos.z + cos_s(rot_y) * -30.0;
    }

    /// `EnDodongo_Destroy`: the trail freed (the colliders: nothing to free).
    fn destroy(&mut self, play: &mut PlayState) {
        play.effect_delete(self.blure_idx);
    }

    /// `EnDodongo_Draw`'s changes to the actor: `EnDodongo_PostLimbDraw` on every limb (the
    /// skeleton's matrices with limbs 15 and 16 swollen), then with ice its colour held on and a
    /// chunk every 4 frames (`EffectSsEnIce_SpawnFlyingVec3f`, at `icePos[iceTimer >> 2]`).
    fn draw_update(&mut self, play: &mut PlayState) {
        if self.actor.killed {
            return;
        }
        let Some(skeleton) = self.skeleton.clone() else { return };
        let model = actor_draw_matrix(&RenderState::of(&self.actor));
        let joints = self.skel.joint_table.clone();
        let body_scale = self.body_scale;
        let mut posts = Vec::new();
        draw_opa_pose(&skeleton, &joints, |limb, _, _| LimbDraw::Default(limb_pre(limb, body_scale)), |limb, m| posts.push((limb, m)));
        for (limb, m) in posts {
            self.post_limb_draw(play, limb, model * m);
        }
        if self.ice_timer != 0 {
            self.actor.color_filter_timer = self.actor.color_filter_timer.wrapping_add(1);
            self.ice_timer -= 1;
            if self.ice_timer % 4 == 0 {
                let index = (self.ice_timer >> 2) as usize;
                let pos = self.ice_pos[index];
                let r = self.actor.shape_rot;
                let me = play.cur_actor.map(|h| oot_game::effect::SsActor { handle: h, world_pos: self.actor.world_pos, shape_rot: [r.x, r.y, r.z] });
                play.effect_ss_en_ice_spawn_flying_vec3f(me, pos, [150, 150, 150, 250], [235, 245, 255], 1.8);
            }
        }
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        rs.values = vec![self.body_scale.x, self.body_scale.y, self.body_scale.z];
        rs
    }

    /// `EnDodongo_Draw`: `Gfx_SetupDL_25Opa` and the skeleton (`SkelAnime_DrawOpa`), limbs 15 and
    /// 16 scaled by `bodyScale` (`EnDodongo_OverrideLimbDraw`).
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let (Some(joints), Some(skeleton)) = (&rs.joints, &self.skeleton) else { return };
        let [x, y, z] = rs.values.as_slice() else { return };
        let body_scale = Vec3::new(*x, *y, *z);
        let bones = skeleton.pose_override(&joints.rot, |limb, _, _| limb_pre(limb, body_scale));
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::mesh(OBJECT, SKEL)), transform: actor_draw_matrix(rs), bones, params: Default::default() });
    }

    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        match id {
            COL_BODY => Some(ColliderMut::JntSph(&mut self.body_collider)),
            COL_HARD => Some(ColliderMut::Tris(&mut self.collider_hard)),
            COL_AT => Some(ColliderMut::Quad(&mut self.collider_at)),
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

impl EnDodongo {
    /// `EnDodongo_AteBomb`: the first explosive (`ACTORCAT_EXPLOSIVE`, from the list's head) of
    /// params 0 with no `parent` within 20 across, 10 up or down, 20 along of its mouth: an
    /// `En_Bom` becomes its `child`, any other its `parent`; the explosive's `parent` it.
    fn ate_bomb(&mut self, play: &mut PlayState) -> bool {
        let mouth = self.mouth_pos;
        let mut found = None;
        for &h in play.actors.category(ACTORCAT_EXPLOSIVE) {
            let Some(a) = play.actors.actor(h) else { continue };
            // schema: another actor's params read whole, as the C does.
            if a.params != 0 || a.parent.is_some() {
                continue;
            }
            let d = a.world_pos - mouth;
            if d.x.abs() < 20.0 && d.y.abs() < 10.0 && d.z.abs() < 20.0 {
                found = Some((h, a.id));
                break;
            }
        }
        let Some((h, id)) = found else { return false };
        if id == ACTOR_EN_BOM {
            self.actor.child = Some(h);
        } else {
            self.actor.parent = Some(h);
        }
        let me = play.cur_actor;
        if let Some(a) = play.actors.actor_mut(h) {
            a.parent = me;
        }
        true
    }
}
