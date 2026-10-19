//! `En_Rd` (`ovl_En_Rd/z_en_rd.c`): the ReDead and the Gibdo, the mummies that freeze Link with
//! a scream and hold him to bite (GAME-07 milestone 6). Child Link meets ReDeads in the royal
//! tomb under the graveyard (three in room 1) and in the ReDead grave.
//!
//! - **Idle** (`EnRd_Idle`): standing (a crying one sobs or wipes its tears), its idle animation
//!   restarting every 10 to 20 frames (the "judder"), crying out every 96 frames. It notices Link
//!   only when he makes a noise (`func_8002DDE4`: `PLAYER_STATE2_3`, a footstep faster than 4, a
//!   sword swing, the sword drawn or put away, an item changed) within 150 across; then it turns
//!   to him (`EnRd_StandUp` first for a crying one).
//! - **The scream** (`EnRd_AttemptPlayerFreeze`): its head and upper body turn towards Link
//!   (`EnRd_TurnTowardsPlayer`, 500 a frame each); within 0x2008 of facing him it screams
//!   (`NA_SE_EN_REDEAD_AIM`): Link frozen 60 frames (Player's `freezeTimer`) and locked on to it
//!   (`Player_SetAutoLockOnActor`), and it walks at him.
//! - **Walking at Link** (`EnRd_WalkToPlayer`, 0.4 a frame): screaming again (40 frames' freeze)
//!   with Link within 150 and 0x1554 of its facing, 60 frames apart; within 45 and facing him
//!   (0x38E3) it **grabs** him (`play->grabPlayer`): Link held (`PLAYER_STATE2_7`), 8 (half a
//!   heart) off at once and every 20 frames, the ReDead behind him (25 back), as low as him for a
//!   child, until he struggles free (`Player_Action_8084F308`: the stick and A or B); then it lets
//!   go (`gGibdoRedeadGrabEndAnim`), can't scream for 10 frames or grab for 15. With Link 150 from
//!   its home or out of reach (dead, climbing, in the air, held), it **walks home**
//!   (`EnRd_WalkToHome`) and stands (crouches) there.
//! - **Hit** (`EnRd_UpdateDamage`, `sDamageTable`): swords, sticks and the hammer hurt it
//!   (red, knocked back 2), Din's Fire burns it (`fireTimer` 40: ten flames over its body,
//!   `EffectSsEnFire`), the hookshot and light magic stun it; 8 health. At 0 it **dies**
//!   (`EnRd_Dead`): every other ReDead that mourns (types 0 and up) gets it as its `parent` and
//!   walks over to stand by it (`EnRd_WalkToParent`; types 1 and up also from walking); it drops
//!   from table 9, lies 300 frames once down, sets its switch flag, then fades (alpha 5 a frame,
//!   sinking), its mourners let go at alpha 180, and goes.
//! - **The Sun's Song** (`gSaveContext.sunsSongState`): every ReDead updating (not rising, hurt,
//!   dying or stunned) is stunned 600 frames, grey (`EnRd_Stunned`); the first to recover ends the
//!   song's state. Any ReDead not stunned by it clears the state at its update.
//! - **A Gibdo rising** (type -3, `EnRd_RiseFromCoffin`): lying flat, it rises 50 out of its coffin,
//!   slides forward, stands up and comes down to its home.
//!
//! The whole overlay is ported. Its draw (`EnRd_Draw`) is the skeleton with its head and upper
//! body turned (`EnRd_OverrideLimbDraw`), opaque at full alpha (recording ten fire positions,
//! `EnRd_PostLimbDraw`) or translucent fading (`gActorSetupXluDL`), its env alpha set; four bakes
//! (each skeleton, opaque and translucent). The invisible type (3) has `ACTOR_FLAG_REACT_TO_LENS`:
//! `Actor_DrawAll` draws it only where the room shows lens actors by default (`lensMode`
//! `LENS_MODE_HIDE_ACTORS`, in its own room); the Lens of Truth isn't ported, so it's never seen
//! elsewhere.
//!
//! Its draw puts its circle shadow down itself (`func_80033C30`, 0.25, at its alpha:
//! `oot_game::shadow::func_80033c30`); the shape has none (`ActorShape_Init(.., NULL, 0)`).
//!
//! Not ported here: `Rumble_Request` (no rumble).
//!
//! Params: `type` bits 0 to 7, signed (`EnRdType`: -3 a Gibdo rising out of its coffin, -2 a
//! Gibdo, -1 a ReDead that never mourns, 0 one that doesn't mourn while walking, 1 a ReDead, 2 one
//! crying, 3 an invisible one (`ACTOR_FLAG_REACT_TO_LENS`)); -1 and up use `gRedeadSkel`, the
//! Gibdos `gGibdoSkel`; init writes the type over the whole params. `rdFlags` bits 8 to 15
//! (`REDEAD_GET_RDFLAGS`): bit 15 (0x80) no scream's freeze; bits 8 to 14 the switch flag set at
//! its death (0x7F in the royal tomb).
//! Flags: reads and sets switch flag `rdFlags & 0x7F` once dead (a flag of 0x40 and up lands on a
//! temp flag by the shift's wrap: 0x7F sets 0x3F); reads and clears `gSaveContext.sunsSongState`.

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use eng_math::{cos_s, sin_s, smooth_step_to_f, smooth_step_to_s, vec3f_yaw};
use glam::{Mat4, Vec3};
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_ENEMY, ACTORCAT_PROP, ActorHandle, ActorImpl, ActorProfile, audio_play_actor_sfx_at, audio_play_actor_sfx2};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind};
use oot_game::audio::sfx::NA_SE_VO_LI_DAMAGE_S;
use oot_game::clock::SUNSSONG_INACTIVE;
use oot_game::collision_check::*;
use oot_game::gbi::Dl;
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::skelanime_std::{ANIMMODE_LOOP, ANIMMODE_LOOP_INTERP, ANIMMODE_ONCE, ANIMMODE_ONCE_INTERP, Anim, SkelAnimeStd};

use crate::player::{
    self, Player, STATE1_13, STATE1_14, STATE1_18, STATE1_19, STATE1_21, STATE1_7, STATE2_7,
};

/// `ACTOR_EN_RD` (`actor_table.h`: 0x0090).
pub const ACTOR_EN_RD: i16 = 0x0090;
pub const OBJECT: &str = "object_rd";
const SKEL_REDEAD: &str = "gRedeadSkel";
const SKEL_GIBDO: &str = "gGibdoSkel";

/// `ACTOR_FLAG_HOOKSHOT_PULLS_PLAYER` (`actor.h`: 1 << 10).
const ACTOR_FLAG_HOOKSHOT_PULLS_PLAYER: u32 = 1 << 10;

/// `En_Rd_Profile`: `ACTORCAT_ENEMY`, attention-enabled, hostile, always updating, the hookshot
/// pulling Link to it.
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_EN_RD,
    name: "En_Rd",
    category: ACTORCAT_ENEMY,
    flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE | ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_HOOKSHOT_PULLS_PLAYER,
    object: OBJECT,
    schema: &SCHEMA,
};

/// The type: params bits 0 to 7 as an `s8` (`EnRd_Init` sign-extends them over the params).
pub const TYPE: Field = Field::params(
    "type",
    0,
    8,
    Kind::Named(&[
        (-3, "REDEAD_TYPE_GIBDO_RISING_OUT_OF_COFFIN"),
        (-2, "REDEAD_TYPE_GIBDO"),
        (-1, "REDEAD_TYPE_DOES_NOT_MOURN"),
        (0, "REDEAD_TYPE_DOES_NOT_MOURN_IF_WALKING"),
        (1, "REDEAD_TYPE_REGULAR"),
        (2, "REDEAD_TYPE_CRYING"),
        (3, "REDEAD_TYPE_INVISIBLE"),
    ]),
    "which mummy, and whether it mourns, cries or hides from the lens",
)
.signed();
/// `REDEAD_GET_RDFLAGS`: `PARAMS_GET_S(params, 8, 8)`.
pub const RD_FLAGS: Field = Field::params("rdFlags", 8, 8, Kind::Range(0, 0xFF), "its scream's freeze (bit 15) and its death's switch flag (bits 8 to 14)");
/// `rdFlags & 0x80`.
pub const NO_FREEZE: Field =
    Field::params("rdFlags & 0x80", 15, 1, Kind::Named(&[(0, "its scream freezes Link"), (1, "its scream doesn't")]), "whether its scream freezes Link").part_of("rdFlags");
/// `rdFlags & 0x7F`.
pub const SWITCH_FLAG: Field =
    Field::params("rdFlags & 0x7F", 8, 7, Kind::SwitchFlag, "set once it's dead (0x40 and up land on a temp flag: 0x7F sets 0x3F)").part_of("rdFlags");

/// En_Rd's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[TYPE, RD_FLAGS, NO_FREEZE, SWITCH_FLAG],
    flags: &[
        FlagUse::new(Flag::Switch("rdFlags & 0x7F")).reads().sets().note("once dead"),
        FlagUse::new(Flag::Other("gSaveContext.sunsSongState")).reads().clears().note("the Sun's Song stuns it"),
    ],
    todo: false,
    objects: &[],
    texts: &[],
};

// The sounds (`include/tables/sfx/enemybank_table.h`; none in `oot_game::audio::sfx` yet).
/// `NA_SE_EN_RIZA_WALK` (0x382E).
pub const NA_SE_EN_RIZA_WALK: u16 = 0x382E;
/// `NA_SE_EN_LIGHT_ARROW_HIT` (0x3836).
pub const NA_SE_EN_LIGHT_ARROW_HIT: u16 = 0x3836;
/// `NA_SE_EN_RIZA_DOWN` (0x387A).
pub const NA_SE_EN_RIZA_DOWN: u16 = 0x387A;
/// `NA_SE_EN_REDEAD_CRY` (0x38E4).
pub const NA_SE_EN_REDEAD_CRY: u16 = 0x38E4;
/// `NA_SE_EN_REDEAD_AIM` (0x38E5).
pub const NA_SE_EN_REDEAD_AIM: u16 = 0x38E5;
/// `NA_SE_EN_REDEAD_DAMAGE` (0x38E6).
pub const NA_SE_EN_REDEAD_DAMAGE: u16 = 0x38E6;
/// `NA_SE_EN_REDEAD_DEAD` (0x38E7).
pub const NA_SE_EN_REDEAD_DEAD: u16 = 0x38E7;
/// `NA_SE_EN_REDEAD_ATTACK` (0x38E8).
pub const NA_SE_EN_REDEAD_ATTACK: u16 = 0x38E8;

/// `NAVI_ENEMY_REDEAD`, `NAVI_ENEMY_GIBDO` (`actor.h`: 0x2A, 0x2D).
pub const NAVI_ENEMY_REDEAD: u8 = 0x2A;
pub const NAVI_ENEMY_GIBDO: u8 = 0x2D;

/// `COLLECTIBLE_DROP_TABLE_9` (`z_en_item00.h`).
const COLLECTIBLE_DROP_TABLE_9: i16 = 9;

/// `EnRdType`.
pub const REDEAD_TYPE_GIBDO_RISING_OUT_OF_COFFIN: i16 = -3;
pub const REDEAD_TYPE_GIBDO: i16 = -2;
pub const REDEAD_TYPE_DOES_NOT_MOURN: i16 = -1;
pub const REDEAD_TYPE_DOES_NOT_MOURN_IF_WALKING: i16 = 0;
pub const REDEAD_TYPE_REGULAR: i16 = 1;
pub const REDEAD_TYPE_CRYING: i16 = 2;
pub const REDEAD_TYPE_INVISIBLE: i16 = 3;

/// `EnRdAction`.
pub const REDEAD_ACTION_IDLE: u8 = 0;
pub const REDEAD_ACTION_STUNNED: u8 = 1;
pub const REDEAD_ACTION_WALK_TO_HOME: u8 = 2;
pub const REDEAD_ACTION_WALK_TO_PARENT: u8 = 3;
pub const REDEAD_ACTION_WALK_TO_PLAYER_OR_RELEASE_GRAB: u8 = 4;
pub const REDEAD_ACTION_STAND_UP: u8 = 5;
pub const REDEAD_ACTION_CROUCH: u8 = 6;
pub const REDEAD_ACTION_ATTEMPT_PLAYER_FREEZE: u8 = 7;
pub const REDEAD_ACTION_GRAB: u8 = 8;
pub const REDEAD_ACTION_DAMAGED: u8 = 9;
pub const REDEAD_ACTION_DEAD: u8 = 10;
pub const REDEAD_ACTION_RISE_FROM_COFFIN: u8 = 11;

/// `EnRdGrabState`.
pub const REDEAD_GRAB_START: u8 = 0;
pub const REDEAD_GRAB_INITIAL_DAMAGE: u8 = 1;
pub const REDEAD_GRAB_ATTACK: u8 = 2;
pub const REDEAD_GRAB_RELEASE: u8 = 3;
pub const REDEAD_GRAB_END: u8 = 4;

/// `EnRdDamageReaction`.
pub const REDEAD_DMG_REACT_NONE: u8 = 0x0;
pub const REDEAD_DMG_REACT_HOOKSHOT: u8 = 0x1;
pub const REDEAD_DMG_REACT_ICE_MAGIC: u8 = 0x6;
pub const REDEAD_DMG_REACT_LIGHT_MAGIC: u8 = 0xD;
pub const REDEAD_DMG_REACT_FIRE_MAGIC: u8 = 0xE;
pub const REDEAD_DMG_REACT_DAMAGE: u8 = 0xF;

/// `RedeadGibdoLimb` (the same 25 limbs in both skeletons).
pub const REDEAD_GIBDO_LIMB_ROOT: usize = 1;
pub const REDEAD_GIBDO_LIMB_LEFT_SHIN: usize = 4;
pub const REDEAD_GIBDO_LIMB_LEFT_FOOT: usize = 6;
pub const REDEAD_GIBDO_LIMB_RIGHT_SHIN: usize = 9;
pub const REDEAD_GIBDO_LIMB_RIGHT_FOOT: usize = 11;
pub const REDEAD_GIBDO_LIMB_UPPER_BODY_ROOT: usize = 12;
pub const REDEAD_GIBDO_LIMB_TORSO: usize = 14;
pub const REDEAD_GIBDO_LIMB_LEFT_HAND: usize = 18;
pub const REDEAD_GIBDO_LIMB_RIGHT_HAND: usize = 22;
pub const REDEAD_GIBDO_LIMB_HEAD_ROOT: usize = 23;
pub const REDEAD_GIBDO_LIMB_HEAD: usize = 24;
pub const REDEAD_GIBDO_LIMB_PELVIS: usize = 25;
pub const REDEAD_GIBDO_LIMB_MAX: usize = 26;

/// `sCylinderInit`: 20 by 70, hit by all but the shield and the mirror's ray (hard: a sword's
/// strike rings), pushed by Player.
const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_HIT0, at_flags: AT_NONE, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_ON | OC1_TYPE_PLAYER, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK1,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_ON | ACELEM_HOOKABLE,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 20, height: 70, y_shift: 0, pos: [0, 0, 0] },
};

/// `sDamageTable`.
pub static S_DAMAGE_TABLE: DamageTable = DamageTable {
    table: [
        dmg_entry(0, REDEAD_DMG_REACT_NONE),        // Deku nut
        dmg_entry(2, REDEAD_DMG_REACT_DAMAGE),      // Deku stick
        dmg_entry(0, REDEAD_DMG_REACT_NONE),        // Slingshot
        dmg_entry(0, REDEAD_DMG_REACT_NONE),        // Explosive
        dmg_entry(0, REDEAD_DMG_REACT_NONE),        // Boomerang
        dmg_entry(0, REDEAD_DMG_REACT_NONE),        // Normal arrow
        dmg_entry(2, REDEAD_DMG_REACT_DAMAGE),      // Hammer swing
        dmg_entry(0, REDEAD_DMG_REACT_HOOKSHOT),    // Hookshot
        dmg_entry(1, REDEAD_DMG_REACT_DAMAGE),      // Kokiri sword
        dmg_entry(2, REDEAD_DMG_REACT_DAMAGE),      // Master sword
        dmg_entry(4, REDEAD_DMG_REACT_DAMAGE),      // Giant's Knife
        dmg_entry(0, REDEAD_DMG_REACT_NONE),        // Fire arrow
        dmg_entry(0, REDEAD_DMG_REACT_NONE),        // Ice arrow
        dmg_entry(0, REDEAD_DMG_REACT_NONE),        // Light arrow
        dmg_entry(0, REDEAD_DMG_REACT_NONE),        // Unk arrow 1
        dmg_entry(0, REDEAD_DMG_REACT_NONE),        // Unk arrow 2
        dmg_entry(0, REDEAD_DMG_REACT_NONE),        // Unk arrow 3
        dmg_entry(4, REDEAD_DMG_REACT_FIRE_MAGIC),  // Fire magic
        dmg_entry(0, REDEAD_DMG_REACT_ICE_MAGIC),   // Ice magic
        dmg_entry(3, REDEAD_DMG_REACT_LIGHT_MAGIC), // Light magic
        dmg_entry(0, REDEAD_DMG_REACT_NONE),        // Shield
        dmg_entry(0, REDEAD_DMG_REACT_NONE),        // Mirror Ray
        dmg_entry(1, REDEAD_DMG_REACT_DAMAGE),      // Kokiri spin
        dmg_entry(4, REDEAD_DMG_REACT_DAMAGE),      // Giant spin
        dmg_entry(2, REDEAD_DMG_REACT_DAMAGE),      // Master spin
        dmg_entry(2, REDEAD_DMG_REACT_DAMAGE),      // Kokiri jump
        dmg_entry(8, REDEAD_DMG_REACT_DAMAGE),      // Giant jump
        dmg_entry(4, REDEAD_DMG_REACT_DAMAGE),      // Master jump
        dmg_entry(0, REDEAD_DMG_REACT_NONE),        // Unknown 1
        dmg_entry(0, REDEAD_DMG_REACT_NONE),        // Unblockable
        dmg_entry(4, REDEAD_DMG_REACT_DAMAGE),      // Hammer jump
        dmg_entry(0, REDEAD_DMG_REACT_NONE),        // Unknown 2
    ],
};

/// The draw's segments: 8 the setup list (`ACTOR_SETUP_OPA_DL`, `&gActorSetupXluDL[2]`: just its
/// end; or `gActorSetupXluDL`), which every limb's list calls; 0xE the env colour
/// (`gDPSetEnvColor(0, 0, 0, alpha)`, run before the skeleton).
const SEG_SETUP: u8 = 0x08;
const SEG_ALPHA: u8 = 0x0E;

/// The bake of a skeleton (`gibdo`), opaque or translucent (`xlu`).
pub fn bake_name(gibdo: bool, xlu: bool) -> String {
    format!("En_Rd/{}_{}", if gibdo { "gibdo" } else { "redead" }, if xlu { "xlu" } else { "opa" })
}

/// `gActorSetupXluDL` (`z_actor.c`): `gsDPSetRenderMode(G_RM_FOG_SHADE_A, G_RM_AA_ZB_XLU_SURF2)`,
/// `gsDPSetAlphaCompare(G_AC_THRESHOLD)`, the end (as `Demo_Sa`'s).
fn actor_setup_xlu_dl() -> Vec<(u32, u32)> {
    let mut d = Dl::default();
    d.0.push((0xE200_001C, 0xC810_49F8));
    d.othermode_l(0, 2, 1);
    d.end();
    d.0
}

pub fn bakes() -> Vec<MeshBake> {
    let mut v = Vec::new();
    for gibdo in [false, true] {
        for xlu in [false, true] {
            v.push(MeshBake {
                name: bake_name(gibdo, xlu),
                object: OBJECT.into(),
                segments: vec![
                    (SEG_SETUP, BakeSegment::Commands(if xlu { actor_setup_xlu_dl() } else { vec![(0xDF00_0000, 0)] })),
                    (SEG_ALPHA, BakeSegment::DynamicColor { env: true, prim: false }),
                ],
                prelude: vec![SEG_ALPHA],
                body: BakeBody::Skeleton { file: OBJECT.into(), symbol: if gibdo { SKEL_GIBDO } else { SKEL_REDEAD }.into(), limbs: Vec::new() },
            });
        }
    }
    v
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Func {
    Idle,
    RiseFromCoffin,
    WalkToPlayer,
    WalkToHome,
    WalkToParent,
    Grab,
    AttemptPlayerFreeze,
    StandUp,
    Crouch,
    Damaged,
    Dead,
    Stunned,
}

impl Func {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Func::Idle => "EnRd_Idle",
            Func::RiseFromCoffin => "EnRd_RiseFromCoffin",
            Func::WalkToPlayer => "EnRd_WalkToPlayer",
            Func::WalkToHome => "EnRd_WalkToHome",
            Func::WalkToParent => "EnRd_WalkToParent",
            Func::Grab => "EnRd_Grab",
            Func::AttemptPlayerFreeze => "EnRd_AttemptPlayerFreeze",
            Func::StandUp => "EnRd_StandUp",
            Func::Crouch => "EnRd_Crouch",
            Func::Damaged => "EnRd_Damaged",
            Func::Dead => "EnRd_Dead",
            Func::Stunned => "EnRd_Stunned",
        }
    }
}

struct Anims {
    grab_attack: Anim,
    grab_end: Anim,
    grab_start: Anim,
    look_back: Anim,
    wiping_tears: Anim,
    sobbing: Anim,
    death: Anim,
    damage: Anim,
    stand_up: Anim,
    idle: Anim,
    walk: Anim,
}

impl Anims {
    fn load(play: &PlayState) -> Option<Anims> {
        let a = play.assets.clone()?;
        let get = |s: &str| a.animation(OBJECT, s).map_err(|e| log::error!("En_Rd: {e:#}")).ok();
        Some(Anims {
            grab_attack: get("gGibdoRedeadGrabAttackAnim")?,
            grab_end: get("gGibdoRedeadGrabEndAnim")?,
            grab_start: get("gGibdoRedeadGrabStartAnim")?,
            look_back: get("gGibdoRedeadLookBackAnim")?,
            wiping_tears: get("gGibdoRedeadWipingTearsAnim")?,
            sobbing: get("gGibdoRedeadSobbingAnim")?,
            death: get("gGibdoRedeadDeathAnim")?,
            damage: get("gGibdoRedeadDamageAnim")?,
            stand_up: get("gGibdoRedeadStandUpAnim")?,
            idle: get("gGibdoRedeadIdleAnim")?,
            walk: get("gGibdoRedeadWalkAnim")?,
        })
    }
}

/// What the ReDead reads of Player (`GET_PLAYER(play)`).
struct PlayerView {
    h: ActorHandle,
    pos: Vec3,
    rot_y: i16,
    state1: u32,
    state2: u32,
    unk_844: i8,
    unk_845: u8,
    unk_92: u16,
}

fn player_view(play: &PlayState) -> Option<PlayerView> {
    let h = play.player?;
    let p = play.actors.downcast::<Player>(h)?;
    Some(PlayerView {
        h,
        pos: p.actor.world_pos,
        rot_y: p.actor.shape_rot.y,
        state1: p.state1,
        state2: p.state2,
        unk_844: p.unk_844,
        unk_845: p.unk_845 as u8,
        unk_92: p.age.climb.unk_92,
    })
}

/// Player out of reach for the scream and the walk to him: dead, or `PLAYER_STATE1_13`, `_14`,
/// `_18`, `_19`, `_21` (hanging, climbing, jumping, falling, on a ladder), or held
/// (`PLAYER_STATE2_7`).
fn player_unreachable(p: &PlayerView) -> bool {
    p.state1 & (STATE1_7 | STATE1_13 | STATE1_14 | STATE1_18 | STATE1_19 | STATE1_21) != 0 || p.state2 & STATE2_7 != 0
}

/// `ABS` of an s16.
fn abs_s(v: i16) -> i32 {
    (v as i32).abs()
}

pub struct EnRd {
    pub actor: Actor,
    /// `firePos`: the ten body parts' places (`Vec3s`) for the flames, from the opaque draw.
    pub fire_pos: [[i16; 3]; 10],
    pub skel: SkelAnimeStd,
    skeleton: Option<Arc<Skeleton>>,
    anims: Option<Anims>,
    pub grab_state: u8,
    pub is_mourning: bool,
    /// `playerStunWaitTimer`: no scream while it's running.
    pub player_stun_wait_timer: u8,
    /// `grabWaitTimer`: no grab while it's running.
    pub grab_wait_timer: u8,
    pub func: Func,
    pub timer: i16,
    pub head_y_rotation: i16,
    pub upper_body_y_rotation: i16,
    pub rd_flags: i16,
    pub alpha: i16,
    pub suns_song_stun_timer: i16,
    pub stunned_by_suns_song: bool,
    pub grab_damage_timer: u8,
    pub fire_timer: u8,
    /// `action` (`EnRdAction`).
    pub action: u8,
    pub damage_reaction: u8,
    /// `unk_31D`: Player's `unk_845` (the swing) at the last hit.
    pub unk_31d: u8,
    pub collider: ColliderCylinder,
    /// The skeleton is `gGibdoSkel`.
    pub gibdo: bool,
}

impl EnRd {
    fn anim(&self, f: impl Fn(&Anims) -> &Anim) -> Option<Anim> {
        self.anims.as_ref().map(|a| f(a).clone())
    }

    /// `EnRd_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // sInitChain: lockOnArrowOffset 2000, scale 0.01, gravity -3.5.
        actor.target_arrow_offset = 2000.0;
        actor.scale = Vec3::splat(0.01);
        actor.gravity = -3.5;
        actor.target_mode = 0; // ATTENTION_RANGE_0
        actor.col_chk_info.damage_table = Some(&S_DAMAGE_TABLE);
        actor.shape_init(0.0, oot_game::shadow::ShadowDraw::None, 0.0);
        actor.focus_pos = actor.world_pos;
        actor.focus_pos.y += 50.0;
        actor.col_chk_info.mass = MASS_HEAVY;
        actor.col_chk_info.health = 8;
        let rd_flags = RD_FLAGS.get(actor.params);
        // PARAMS_GET_NOSHIFT(params, 7, 1): params |= 0xFF00, else params &= 0xFF; the type
        // sign-extended.
        actor.params = TYPE.get(actor.params);
        let gibdo = TYPE.get(actor.params) < REDEAD_TYPE_DOES_NOT_MOURN;
        let skel_name = if gibdo { SKEL_GIBDO } else { SKEL_REDEAD };
        let skeleton = play.assets.clone().and_then(|a| a.skeleton(OBJECT, skel_name).map_err(|e| log::error!("En_Rd: {e:#}")).ok());
        let anims = Anims::load(play);
        // SkelAnime_InitFlex(.., gGibdoRedeadIdleAnim, .., REDEAD_GIBDO_LIMB_MAX).
        let skel = SkelAnimeStd::init_flex(REDEAD_GIBDO_LIMB_MAX - 1, anims.as_ref().map(|a| a.idle.clone()));
        actor.navi_enemy_id = if gibdo { NAVI_ENEMY_GIBDO } else { NAVI_ENEMY_REDEAD };
        let mut this = EnRd {
            actor,
            fire_pos: [[0; 3]; 10],
            skel,
            skeleton,
            anims,
            grab_state: 0,
            is_mourning: false,
            player_stun_wait_timer: 0,
            grab_wait_timer: 0,
            func: Func::Idle,
            timer: 0,
            head_y_rotation: 0,
            upper_body_y_rotation: 0,
            rd_flags,
            alpha: 255,
            suns_song_stun_timer: 0,
            stunned_by_suns_song: false,
            grab_damage_timer: 0,
            fire_timer: 0,
            action: REDEAD_ACTION_IDLE,
            damage_reaction: 0,
            unk_31d: 255,
            collider: ColliderCylinder::new(&CYLINDER_INIT),
            gibdo,
        };
        if TYPE.get(this.actor.params) >= REDEAD_TYPE_GIBDO {
            this.setup_idle(play);
        } else {
            this.setup_rise_from_coffin();
        }
        this.skel.update();
        if TYPE.get(this.actor.params) == REDEAD_TYPE_INVISIBLE {
            this.actor.flags |= ACTOR_FLAG_REACT_TO_LENS;
        }
        Box::new(this)
    }

    /// `EnRd_UpdateMourningTarget`: every other mourning ReDead (`params` 0 and up) in the enemy
    /// list gets this one as its `parent` (`should_mourn`), or lets go of it.
    fn update_mourning_target(play: &mut PlayState, me: Option<ActorHandle>, should_mourn: bool) {
        let Some(me) = me else { return };
        let list = play.actors.category(ACTORCAT_ENEMY).to_vec();
        for h in list {
            if h == me {
                continue;
            }
            let Some(a) = play.actors.actor_mut(h) else { continue };
            if a.id != ACTOR_EN_RD || TYPE.get(a.params) < REDEAD_TYPE_DOES_NOT_MOURN_IF_WALKING {
                continue;
            }
            if should_mourn {
                a.parent = Some(me);
            } else if a.parent == Some(me) {
                a.parent = None;
            }
        }
    }

    /// `EnRd_SetupIdle`.
    fn setup_idle(&mut self, play: &mut PlayState) {
        if TYPE.get(self.actor.params) != REDEAD_TYPE_CRYING {
            // Animation_MorphToLoop(gGibdoRedeadIdleAnim, -6).
            if let Some(a) = self.anim(|a| &a.idle) {
                self.skel.change(a, 1.0, 0.0, 0.0, ANIMMODE_LOOP, -6.0);
            }
        } else if let Some(a) = self.anim(|a| &a.sobbing) {
            let last = a.last_frame();
            self.skel.change(a, 1.0, 0.0, last, ANIMMODE_LOOP, 0.0);
        }
        self.action = REDEAD_ACTION_IDLE;
        self.timer = ((play.rand.zero_one() * 10.0) + 5.0) as i16;
        self.actor.speed_xz = 0.0;
        self.actor.world_rot.y = self.actor.shape_rot.y;
        self.func = Func::Idle;
    }

    /// `EnRd_Idle`.
    fn idle(&mut self, play: &mut PlayState) {
        self.skel.update();
        smooth_step_to_s(&mut self.head_y_rotation, 0, 1, 0x64, 0);
        smooth_step_to_s(&mut self.upper_body_y_rotation, 0, 1, 0x64, 0);
        if TYPE.get(self.actor.params) == REDEAD_TYPE_CRYING && self.skel.cur_frame == 0.0 {
            let pick = if play.rand.zero_one() >= 0.5 { self.anim(|a| &a.sobbing) } else { self.anim(|a| &a.wiping_tears) };
            if let Some(a) = pick {
                let last = a.last_frame();
                self.skel.change(a, 1.0, 0.0, last, ANIMMODE_LOOP, 0.0);
            }
        } else {
            self.timer -= 1;
            if self.timer == 0 {
                // The idle animation back to its first frame: the "judder" in place.
                self.timer = ((play.rand.zero_one() * 10.0) + 10.0) as i16;
                self.skel.cur_frame = 0.0;
            }
        }
        if self.actor.parent.is_some() {
            if !self.is_mourning {
                if TYPE.get(self.actor.params) != REDEAD_TYPE_CRYING {
                    self.setup_walk_to_parent();
                } else {
                    self.setup_stand_up();
                }
            }
        } else {
            if self.is_mourning {
                if TYPE.get(self.actor.params) != REDEAD_TYPE_CRYING {
                    self.setup_attempt_player_freeze();
                } else {
                    self.setup_stand_up();
                }
            }
            self.is_mourning = false;
            // func_8002DDE4: Player made a noise this frame (PLAYER_STATE2_3).
            let noisy = player_view(play).is_some_and(|p| p.state2 & player::STATE2_3 != 0);
            if self.actor.xz_dist_to_player <= 150.0 && noisy {
                if TYPE.get(self.actor.params) != REDEAD_TYPE_CRYING && !self.is_mourning {
                    self.setup_attempt_player_freeze();
                } else {
                    self.setup_stand_up();
                }
            }
        }
        if play.gameplay_frames & 0x5F == 0 {
            audio_play_actor_sfx2(play, NA_SE_EN_REDEAD_CRY);
        }
    }

    /// `EnRd_SetupRiseFromCoffin`: flat on its back (pitch -0x4000), the idle animation held
    /// (speed 0), no gravity.
    fn setup_rise_from_coffin(&mut self) {
        if let Some(a) = self.anim(|a| &a.idle) {
            let last = a.last_frame();
            self.skel.change(a, 0.0, 0.0, last, ANIMMODE_LOOP, -6.0);
        }
        self.action = REDEAD_ACTION_RISE_FROM_COFFIN;
        self.timer = 6;
        self.actor.shape_rot.x = -0x4000;
        self.actor.gravity = 0.0;
        self.actor.shape_y_offset = 0.0;
        self.actor.speed_xz = 0.0;
        self.func = Func::RiseFromCoffin;
    }

    /// `EnRd_RiseFromCoffin`: up 50 (crying out as it starts), forward at up to 6 for 6 frames,
    /// slowing; stopped, it stands up (0x7D0 a frame) and comes down to its home's height.
    fn rise_from_coffin(&mut self, play: &mut PlayState) {
        if self.actor.shape_rot.x != -0x4000 {
            smooth_step_to_s(&mut self.actor.shape_rot.x, 0, 1, 0x7D0, 0);
            if smooth_step_to_f(&mut self.actor.world_pos.y, self.actor.home_pos.y, 0.3, 2.0, 0.3) == 0.0 {
                self.actor.gravity = -3.5;
                self.setup_idle(play);
            }
        } else {
            if self.actor.world_pos.y == self.actor.home_pos.y {
                audio_play_actor_sfx2(play, NA_SE_EN_REDEAD_CRY);
            }
            if smooth_step_to_f(&mut self.actor.world_pos.y, self.actor.home_pos.y + 50.0, 0.3, 2.0, 0.3) == 0.0 {
                if self.timer != 0 {
                    self.timer -= 1;
                    smooth_step_to_f(&mut self.actor.speed_xz, 6.0, 0.3, 1.0, 0.3);
                } else if smooth_step_to_f(&mut self.actor.speed_xz, 0.0, 0.3, 1.0, 0.3) == 0.0 {
                    smooth_step_to_s(&mut self.actor.shape_rot.x, 0, 1, 0x7D0, 0);
                }
            }
        }
    }

    /// `EnRd_SetupWalkToPlayer`.
    fn setup_walk_to_player(&mut self) {
        if let Some(a) = self.anim(|a| &a.walk) {
            let last = a.last_frame();
            self.skel.change(a, 1.0, 4.0, last, ANIMMODE_LOOP_INTERP, -4.0);
        }
        self.actor.speed_xz = 0.4;
        self.action = REDEAD_ACTION_WALK_TO_PLAYER_OR_RELEASE_GRAB;
        self.func = Func::WalkToPlayer;
    }

    /// The walk's footsteps at frames 10 and 22, else its cry every 96 frames.
    fn walk_sfx(&self, play: &mut PlayState) {
        if self.skel.cur_frame == 10.0 || self.skel.cur_frame == 22.0 {
            audio_play_actor_sfx2(play, NA_SE_EN_RIZA_WALK);
        } else if play.gameplay_frames & 0x5F == 0 {
            audio_play_actor_sfx2(play, NA_SE_EN_REDEAD_CRY);
        }
    }

    /// Player's freeze (`player->actor.freezeTimer`).
    fn set_player_freeze_timer(play: &mut PlayState, h: ActorHandle, timer: u16) {
        if let Some(a) = play.actors.actor_mut(h) {
            a.freeze_timer = timer;
        }
    }

    /// `EnRd_WalkToPlayer`.
    fn walk_to_player(&mut self, play: &mut PlayState) {
        let Some(p) = player_view(play) else { return };
        let yaw = self.actor.yaw_towards_player.wrapping_sub(self.actor.shape_rot.y).wrapping_sub(self.head_y_rotation).wrapping_sub(self.upper_body_y_rotation);
        self.skel.play_speed = self.actor.speed_xz;
        smooth_step_to_s(&mut self.actor.shape_rot.y, self.actor.yaw_towards_player, 1, 0xFA, 0);
        smooth_step_to_s(&mut self.head_y_rotation, 0, 1, 0x64, 0);
        smooth_step_to_s(&mut self.upper_body_y_rotation, 0, 1, 0x64, 0);
        self.actor.world_rot.y = self.actor.shape_rot.y;
        self.skel.update();
        if p.pos.distance(self.actor.home_pos) >= 150.0 {
            self.setup_walk_to_home();
        }
        if abs_s(yaw) < 0x1554 && self.actor.world_pos.distance(p.pos) <= 150.0 {
            if !player_unreachable(&p) {
                if self.player_stun_wait_timer == 0 {
                    if self.rd_flags & 0x80 == 0 {
                        Self::set_player_freeze_timer(play, p.h, 40);
                        // Player can't update while frozen, so no reticle (zTargetActiveTimer),
                        // but the camera turns to the ReDead; the C sets autoLockOnActor again
                        // (redundant).
                        if let Some(me) = play.cur_actor {
                            player::player_set_auto_lock_on_actor(play, me);
                        }
                        // Rumble_Request(xzDistToPlayer, 255, 20, 150): no rumble.
                    }
                    self.player_stun_wait_timer = 60;
                    audio_play_actor_sfx2(play, NA_SE_EN_REDEAD_AIM);
                }
            } else {
                self.setup_walk_to_home();
            }
        }
        if self.grab_wait_timer != 0 {
            self.grab_wait_timer -= 1;
        }
        // Actor_IsFacingPlayer(&this->actor, 0x38E3).
        let facing = abs_s(self.actor.yaw_towards_player.wrapping_sub(self.actor.shape_rot.y)) < 0x38E3;
        if self.grab_wait_timer == 0 && self.actor.world_pos.distance(p.pos) <= 45.0 && facing {
            Self::set_player_freeze_timer(play, p.h, 0);
            if player::play_grab_player(play) {
                self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
                self.setup_grab();
            }
        } else if TYPE.get(self.actor.params) > REDEAD_TYPE_DOES_NOT_MOURN_IF_WALKING {
            if self.actor.parent.is_some() {
                self.setup_walk_to_parent();
            } else {
                self.is_mourning = false;
            }
        }
        self.walk_sfx(play);
    }

    /// `EnRd_SetupWalkToHome`: the walk at half speed (its `speed` as it was).
    fn setup_walk_to_home(&mut self) {
        if let Some(a) = self.anim(|a| &a.walk) {
            let last = a.last_frame();
            self.skel.change(a, 0.5, 0.0, last, ANIMMODE_LOOP_INTERP, -4.0);
        }
        self.action = REDEAD_ACTION_WALK_TO_HOME;
        self.func = Func::WalkToHome;
    }

    /// `EnRd_WalkToHome`.
    fn walk_to_home(&mut self, play: &mut PlayState) {
        let target_y = vec3f_yaw(self.actor.world_pos, self.actor.home_pos);
        if self.actor.world_pos.distance(self.actor.home_pos) >= 5.0 {
            smooth_step_to_s(&mut self.actor.shape_rot.y, target_y, 1, 0x1C2, 0);
        } else {
            self.actor.speed_xz = 0.0;
            if smooth_step_to_s(&mut self.actor.shape_rot.y, self.actor.home_rot.y, 1, 0x1C2, 0) == 0 {
                if TYPE.get(self.actor.params) != REDEAD_TYPE_CRYING {
                    self.setup_idle(play);
                } else {
                    self.setup_crouch();
                }
            }
        }
        smooth_step_to_s(&mut self.head_y_rotation, 0, 1, 0x64, 0);
        smooth_step_to_s(&mut self.upper_body_y_rotation, 0, 1, 0x64, 0);
        self.actor.world_rot.y = self.actor.shape_rot.y;
        self.skel.update();
        let reachable = player_view(play).is_some_and(|p| !player_unreachable(&p) && p.pos.distance(self.actor.home_pos) < 150.0);
        if reachable {
            self.actor.target_mode = 0; // ATTENTION_RANGE_0
            self.setup_walk_to_player();
        } else if TYPE.get(self.actor.params) > REDEAD_TYPE_DOES_NOT_MOURN_IF_WALKING {
            if self.actor.parent.is_some() {
                self.setup_walk_to_parent();
            } else {
                self.is_mourning = false;
            }
        }
        self.walk_sfx(play);
    }

    /// `EnRd_SetupWalkToParent`.
    fn setup_walk_to_parent(&mut self) {
        if let Some(a) = self.anim(|a| &a.walk) {
            let last = a.last_frame();
            self.skel.change(a, 0.5, 0.0, last, ANIMMODE_LOOP_INTERP, -4.0);
        }
        self.action = REDEAD_ACTION_WALK_TO_PARENT;
        self.is_mourning = true;
        self.func = Func::WalkToParent;
    }

    /// `EnRd_WalkToParent`: to the dead one (its `parent`) at 0.4, standing (crouching) within 45
    /// of it; with no parent, at Link. (A parent gone from the arena reads as none.)
    fn walk_to_parent(&mut self, play: &mut PlayState) {
        let parent_pos = self.actor.parent.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos);
        if let Some(parent_pos) = parent_pos {
            let target_y = vec3f_yaw(self.actor.world_pos, parent_pos);
            smooth_step_to_s(&mut self.actor.shape_rot.y, target_y, 1, 0xFA, 0);
            if self.actor.world_pos.distance(parent_pos) >= 45.0 {
                self.actor.speed_xz = 0.4;
            } else {
                self.actor.speed_xz = 0.0;
                if TYPE.get(self.actor.params) != REDEAD_TYPE_CRYING {
                    self.setup_idle(play);
                } else {
                    self.setup_crouch();
                }
            }
            smooth_step_to_s(&mut self.head_y_rotation, 0, 1, 0x64, 0);
            smooth_step_to_s(&mut self.upper_body_y_rotation, 0, 1, 0x64, 0);
        } else {
            self.setup_walk_to_player();
        }
        self.actor.world_rot.y = self.actor.shape_rot.y;
        self.skel.update();
        self.walk_sfx(play);
    }

    /// `EnRd_SetupGrab`.
    fn setup_grab(&mut self) {
        if let Some(a) = self.anim(|a| &a.grab_start) {
            self.skel.play_once(a);
        }
        self.timer = 0;
        self.grab_state = 0;
        self.grab_damage_timer = 200;
        self.action = REDEAD_ACTION_GRAB;
        self.actor.speed_xz = 0.0;
        self.func = Func::Grab;
    }

    /// `EnRd_Grab`: the grab's start; at its end 8 off Link and the bite looping, 8 off every 20
    /// frames, behind Link (25 back, 10 a frame, as low as a child); Link free (`PLAYER_STATE2_7`
    /// off), the release, then back to walking at him.
    fn grab(&mut self, play: &mut PlayState) {
        let Some(p) = player_view(play) else { return };
        if self.skel.update() {
            self.grab_state += 1;
        }
        let adult = play.save.adult;
        let mut state = self.grab_state;
        if state == REDEAD_GRAB_INITIAL_DAMAGE {
            if let Some(a) = self.anim(|a| &a.grab_attack) {
                let last = a.last_frame();
                self.skel.change(a, 1.0, 0.0, last, ANIMMODE_LOOP, 0.0);
            }
            self.grab_state += 1;
            player::play_damage_player(play, -8);
            // Rumble_Request(xzDistToPlayer, 255, 1, 12): no rumble.
            self.grab_damage_timer = 20;
            state = REDEAD_GRAB_START; // FALLTHROUGH
        }
        if state == REDEAD_GRAB_START {
            smooth_step_to_s(&mut self.head_y_rotation, 0, 1, 0x5DC, 0);
            smooth_step_to_s(&mut self.upper_body_y_rotation, 0, 1, 0x5DC, 0);
            state = REDEAD_GRAB_ATTACK; // FALLTHROUGH
        }
        match state {
            REDEAD_GRAB_ATTACK => {
                // Player as it is now (the damage may have let him go).
                let state2 = play.actors.downcast::<Player>(p.h).map(|pl| pl.state2).unwrap_or(0);
                if state2 & STATE2_7 == 0 {
                    if let Some(a) = self.anim(|a| &a.grab_end) {
                        let last = a.last_frame();
                        self.skel.change(a, 0.5, 0.0, last, ANIMMODE_ONCE_INTERP, 0.0);
                    }
                    self.grab_state += 1;
                    self.action = REDEAD_ACTION_WALK_TO_PLAYER_OR_RELEASE_GRAB;
                    return;
                }
                if !adult {
                    smooth_step_to_f(&mut self.actor.shape_y_offset, -1500.0, 1.0, 150.0, 0.0);
                }
                smooth_step_to_f(&mut self.actor.world_pos.x, (sin_s(p.rot_y) * -25.0) + p.pos.x, 1.0, 10.0, 0.0);
                smooth_step_to_f(&mut self.actor.world_pos.y, p.pos.y, 1.0, 10.0, 0.0);
                smooth_step_to_f(&mut self.actor.world_pos.z, (cos_s(p.rot_y) * -25.0) + p.pos.z, 1.0, 10.0, 0.0);
                smooth_step_to_s(&mut self.actor.shape_rot.y, p.rot_y, 1, 0x1770, 0);
                if self.skel.cur_frame == 0.0 {
                    audio_play_actor_sfx2(play, NA_SE_EN_REDEAD_ATTACK);
                }
                self.grab_damage_timer = self.grab_damage_timer.wrapping_sub(1);
                if self.grab_damage_timer == 0 {
                    player::play_damage_player(play, -8);
                    // Rumble_Request(xzDistToPlayer, 240, 1, 12): no rumble.
                    self.grab_damage_timer = 20;
                    // Player_PlaySfx(player, NA_SE_VO_LI_DAMAGE_S + ageProperties->unk_92).
                    audio_play_actor_sfx_at(play, p.h, NA_SE_VO_LI_DAMAGE_S.wrapping_add(p.unk_92));
                }
            }
            REDEAD_GRAB_RELEASE => {
                if !adult {
                    smooth_step_to_f(&mut self.actor.shape_y_offset, 0.0, 1.0, 400.0, 0.0);
                }
            }
            REDEAD_GRAB_END => {
                if !adult {
                    smooth_step_to_f(&mut self.actor.shape_y_offset, 0.0, 1.0, 400.0, 0.0);
                }
                self.actor.target_mode = 0; // ATTENTION_RANGE_0
                self.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED;
                self.player_stun_wait_timer = 0xA;
                self.grab_wait_timer = 0xF;
                self.setup_walk_to_player();
            }
            _ => {}
        }
    }

    /// `EnRd_SetupAttemptPlayerFreeze`: the look back held at its first frame.
    fn setup_attempt_player_freeze(&mut self) {
        if let Some(a) = self.anim(|a| &a.look_back) {
            let last = a.last_frame();
            self.skel.change(a, 0.0, 0.0, last, ANIMMODE_ONCE, 0.0);
        }
        self.action = REDEAD_ACTION_ATTEMPT_PLAYER_FREEZE;
        self.func = Func::AttemptPlayerFreeze;
    }

    /// `EnRd_AttemptPlayerFreeze`: once its head and body face Link within 0x2008, the scream:
    /// Link frozen 60 frames and locked on (unless `rdFlags & 0x80`), then at him.
    fn attempt_player_freeze(&mut self, play: &mut PlayState) {
        let yaw = self.actor.yaw_towards_player.wrapping_sub(self.actor.shape_rot.y).wrapping_sub(self.head_y_rotation).wrapping_sub(self.upper_body_y_rotation);
        if abs_s(yaw) < 0x2008 {
            if self.rd_flags & 0x80 == 0 {
                if let Some(h) = play.player {
                    Self::set_player_freeze_timer(play, h, 60);
                }
                // Rumble_Request(xzDistToPlayer, 255, 20, 150): no rumble.
                if let Some(me) = play.cur_actor {
                    player::player_set_auto_lock_on_actor(play, me);
                }
            }
            audio_play_actor_sfx2(play, NA_SE_EN_REDEAD_AIM);
            self.setup_walk_to_player();
        }
    }

    /// `EnRd_SetupStandUp`.
    fn setup_stand_up(&mut self) {
        // Animation_MorphToPlayOnce(gGibdoRedeadStandUpAnim, -4).
        if let Some(a) = self.anim(|a| &a.stand_up) {
            let last = a.last_frame();
            self.skel.change(a, 1.0, 0.0, last, ANIMMODE_ONCE, -4.0);
        }
        self.action = REDEAD_ACTION_STAND_UP;
        self.func = Func::StandUp;
    }

    /// `EnRd_StandUp`.
    fn stand_up(&mut self) {
        if self.skel.update() {
            if self.actor.parent.is_some() {
                self.setup_walk_to_parent();
            } else {
                self.setup_attempt_player_freeze();
            }
        }
    }

    /// `EnRd_SetupCrouch`: the standing up backwards.
    fn setup_crouch(&mut self) {
        if let Some(a) = self.anim(|a| &a.stand_up) {
            let last = a.last_frame();
            self.skel.change(a, -1.0, last, 0.0, ANIMMODE_ONCE, -4.0);
        }
        self.action = REDEAD_ACTION_CROUCH;
        self.func = Func::Crouch;
    }

    /// `EnRd_Crouch`.
    fn crouch(&mut self, play: &mut PlayState) {
        if self.skel.update() {
            self.setup_idle(play);
        }
    }

    /// `EnRd_SetupDamaged`.
    fn setup_damaged(&mut self, play: &mut PlayState) {
        // Animation_MorphToPlayOnce(gGibdoRedeadDamageAnim, -6).
        if let Some(a) = self.anim(|a| &a.damage) {
            let last = a.last_frame();
            self.skel.change(a, 1.0, 0.0, last, ANIMMODE_ONCE, -6.0);
        }
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
            self.actor.speed_xz = -2.0;
        }
        self.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED;
        audio_play_actor_sfx2(play, NA_SE_EN_REDEAD_DAMAGE);
        self.action = REDEAD_ACTION_DAMAGED;
        self.func = Func::Damaged;
    }

    /// `EnRd_Damaged`.
    fn damaged(&mut self, play: &mut PlayState) {
        if self.actor.speed_xz < 0.0 {
            self.actor.speed_xz += 0.15;
        }
        self.actor.world_rot.y = self.actor.yaw_towards_player;
        smooth_step_to_s(&mut self.head_y_rotation, 0, 1, 0x12C, 0);
        smooth_step_to_s(&mut self.upper_body_y_rotation, 0, 1, 0x12C, 0);
        if self.skel.update() {
            self.actor.world_rot.y = self.actor.shape_rot.y;
            let player_pos = player_view(play).map(|p| p.pos).unwrap_or(self.actor.world_pos);
            if self.actor.parent.is_some() {
                self.setup_walk_to_parent();
            } else if player_pos.distance(self.actor.home_pos) >= 150.0 {
                self.setup_walk_to_home();
            } else {
                self.setup_walk_to_player();
            }
            self.unk_31d = 0xFF;
        }
    }

    /// `EnRd_SetupDead`.
    fn setup_dead(&mut self, play: &mut PlayState) {
        // Animation_MorphToPlayOnce(gGibdoRedeadDeathAnim, -1).
        if let Some(a) = self.anim(|a| &a.death) {
            let last = a.last_frame();
            self.skel.change(a, 1.0, 0.0, last, ANIMMODE_ONCE, -1.0);
        }
        self.action = REDEAD_ACTION_DEAD;
        self.timer = 300;
        self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        self.actor.speed_xz = 0.0;
        audio_play_actor_sfx2(play, NA_SE_EN_REDEAD_DEAD);
        self.func = Func::Dead;
    }

    /// The switch flag set at its death: `rdFlags & 0x7F` (the royal tomb's 0x7F lands on temp
    /// flag 0x3F: `SceneFlags::set_switch`'s wrap, @bug (game)).
    fn death_switch_flag(&self) -> i32 {
        (self.rd_flags & 0x7F) as i32
    }

    /// `EnRd_Dead`: a prop now; the death's falls at frames 33 and 40; once down, 300 frames, its
    /// switch flag, then fading 5 a frame and sinking (its mourners let go at 180), then gone.
    fn dead(&mut self, play: &mut PlayState) {
        if self.actor.category != ACTORCAT_PROP {
            // Actor_ChangeCategory(play, &play->actorCtx, &this->actor, ACTORCAT_PROP).
            self.actor.category = ACTORCAT_PROP;
            if let Some(me) = play.cur_actor {
                play.actors.change_category(me, ACTORCAT_PROP);
            }
        }
        smooth_step_to_s(&mut self.head_y_rotation, 0, 1, 0x7D0, 0);
        smooth_step_to_s(&mut self.upper_body_y_rotation, 0, 1, 0x7D0, 0);
        if self.skel.update() {
            if self.timer == 0 {
                let flag = self.death_switch_flag();
                if !play.flags.get_switch(flag) {
                    play.flags.set_switch(flag);
                }
                if self.alpha != 0 {
                    if self.alpha == 180 {
                        Self::update_mourning_target(play, play.cur_actor, false);
                    }
                    self.actor.scale.y -= 0.000075;
                    self.alpha -= 5;
                } else {
                    self.actor.kill();
                }
            } else {
                self.timer -= 1;
            }
        } else if self.skel.cur_frame as i32 == 33 || self.skel.cur_frame as i32 == 40 {
            audio_play_actor_sfx2(play, NA_SE_EN_RIZA_DOWN);
        }
    }

    /// `EnRd_SetupStunned`: by the Sun's Song 600 frames, grey; by the hookshot blue (80);
    /// otherwise (light magic) grey (80).
    fn setup_stunned(&mut self, play: &mut PlayState) {
        self.action = REDEAD_ACTION_STUNNED;
        self.actor.speed_xz = 0.0;
        self.actor.world_rot.y = self.actor.shape_rot.y;
        if play.save.suns_song_state != SUNSSONG_INACTIVE {
            self.stunned_by_suns_song = true;
            self.suns_song_stun_timer = 600;
            audio_play_actor_sfx2(play, NA_SE_EN_LIGHT_ARROW_HIT);
            self.actor.set_color_filter(COLORFILTER_COLORFLAG_GRAY, COLORFILTER_INTENSITY_FLAG | 200, COLORFILTER_BUFFLAG_OPA, 255);
        } else if self.damage_reaction == REDEAD_DMG_REACT_HOOKSHOT {
            self.actor.set_color_filter(COLORFILTER_COLORFLAG_BLUE, 200, COLORFILTER_BUFFLAG_OPA, 80);
        } else {
            audio_play_actor_sfx2(play, NA_SE_EN_LIGHT_ARROW_HIT);
            self.actor.set_color_filter(COLORFILTER_COLORFLAG_GRAY, 200, COLORFILTER_BUFFLAG_OPA, 80);
        }
        self.func = Func::Stunned;
    }

    /// `EnRd_Stunned`: the Sun's Song's 600 frames counted down (its grey kept at 255 to the last
    /// 255), its end clearing the song's state; the colour gone, dead or hurt.
    fn stunned(&mut self, play: &mut PlayState) {
        if self.stunned_by_suns_song && self.suns_song_stun_timer != 0 {
            self.suns_song_stun_timer -= 1;
            if self.suns_song_stun_timer >= 255 {
                self.actor.set_color_filter(COLORFILTER_COLORFLAG_GRAY, 200, COLORFILTER_BUFFLAG_OPA, 255);
            }
            if self.suns_song_stun_timer == 0 {
                self.stunned_by_suns_song = false;
                play.save.suns_song_state = SUNSSONG_INACTIVE;
            }
        }
        if self.actor.color_filter_timer == 0 {
            if self.actor.col_chk_info.health == 0 {
                Self::update_mourning_target(play, play.cur_actor, true);
                self.setup_dead(play);
                let pos = self.actor.world_pos;
                crate::en_item00::item_drop_collectible_random(play, Some(&self.actor), pos, COLLECTIBLE_DROP_TABLE_9 * 16);
            } else {
                self.setup_damaged(play);
            }
        }
    }

    /// `EnRd_TurnTowardsPlayer`: the upper body and head 500 a frame each towards Link (to
    /// 0x495F and 0x256F).
    fn turn_towards_player(&mut self) {
        let mut head_angle_temp = self.actor.yaw_towards_player.wrapping_sub(self.actor.shape_rot.y.wrapping_add(self.upper_body_y_rotation));
        let upper_body_angle = head_angle_temp.clamp(-500, 500);
        head_angle_temp = head_angle_temp.wrapping_sub(self.head_y_rotation);
        let head_angle = head_angle_temp.clamp(-500, 500);
        if self.actor.yaw_towards_player.wrapping_sub(self.actor.shape_rot.y) >= 0 {
            self.upper_body_y_rotation = (self.upper_body_y_rotation as i32 + abs_s(upper_body_angle)) as i16;
            self.head_y_rotation = (self.head_y_rotation as i32 + abs_s(head_angle)) as i16;
        } else {
            self.upper_body_y_rotation = (self.upper_body_y_rotation as i32 - abs_s(upper_body_angle)) as i16;
            self.head_y_rotation = (self.head_y_rotation as i32 - abs_s(head_angle)) as i16;
        }
        self.upper_body_y_rotation = self.upper_body_y_rotation.clamp(-0x495F, 0x495F);
        self.head_y_rotation = self.head_y_rotation.clamp(-0x256F, 0x256F);
    }

    /// `EnRd_UpdateDamage`: the Sun's Song stuns it (standing, not hurt, dying or stunned); a hit
    /// (not while rising): the hookshot and light magic stun it, the others with a reaction hurt it
    /// (fire 40 frames' flames), to death at 0.
    fn update_damage(&mut self, play: &mut PlayState) {
        if play.save.suns_song_state != SUNSSONG_INACTIVE
            && self.actor.shape_rot.x == 0
            && !self.stunned_by_suns_song
            && self.action != REDEAD_ACTION_DAMAGED
            && self.action != REDEAD_ACTION_DEAD
            && self.action != REDEAD_ACTION_STUNNED
        {
            self.setup_stunned(play);
            return;
        }
        if self.collider.base.ac_flags & AC_HIT == 0 {
            return;
        }
        self.collider.base.ac_flags &= !AC_HIT;
        self.damage_reaction = self.actor.col_chk_info.damage_reaction;
        if self.action == REDEAD_ACTION_RISE_FROM_COFFIN {
            return;
        }
        let elem = self.collider.info;
        self.actor.set_drop_flag(&elem, true);
        if let Some(p) = player_view(play)
            && p.unk_844 != 0
        {
            self.unk_31d = p.unk_845;
        }
        if self.damage_reaction == REDEAD_DMG_REACT_NONE || self.damage_reaction == REDEAD_DMG_REACT_ICE_MAGIC {
            return;
        }
        if (self.damage_reaction == REDEAD_DMG_REACT_HOOKSHOT || self.damage_reaction == REDEAD_DMG_REACT_LIGHT_MAGIC) && self.action != REDEAD_ACTION_STUNNED {
            self.actor.apply_damage();
            self.setup_stunned(play);
            return;
        }
        self.stunned_by_suns_song = false;
        self.suns_song_stun_timer = 0;
        if self.damage_reaction == REDEAD_DMG_REACT_FIRE_MAGIC {
            self.actor.set_color_filter(COLORFILTER_COLORFLAG_RED, 255, COLORFILTER_BUFFLAG_OPA, 80);
            self.fire_timer = 40;
        } else {
            self.actor.set_color_filter(COLORFILTER_COLORFLAG_RED, 255, COLORFILTER_BUFFLAG_OPA, 8);
        }
        self.actor.apply_damage();
        if self.actor.col_chk_info.health == 0 {
            Self::update_mourning_target(play, play.cur_actor, true);
            self.setup_dead(play);
            let pos = self.actor.world_pos;
            crate::en_item00::item_drop_collectible_random(play, None, pos, COLLECTIBLE_DROP_TABLE_9 * 16);
        } else {
            self.setup_damaged(play);
        }
    }

    fn run_action(&mut self, play: &mut PlayState) {
        match self.func {
            Func::Idle => self.idle(play),
            Func::RiseFromCoffin => self.rise_from_coffin(play),
            Func::WalkToPlayer => self.walk_to_player(play),
            Func::WalkToHome => self.walk_to_home(play),
            Func::WalkToParent => self.walk_to_parent(play),
            Func::Grab => self.grab(play),
            Func::AttemptPlayerFreeze => self.attempt_player_freeze(play),
            Func::StandUp => self.stand_up(),
            Func::Crouch => self.crouch(play),
            Func::Damaged => self.damaged(play),
            Func::Dead => self.dead(play),
            Func::Stunned => self.stunned(play),
        }
    }

    /// The skeleton's pose with `EnRd_OverrideLimbDraw`'s turns: the head root by `head`, the
    /// upper body's root by `upper`.
    fn pose(skeleton: &Skeleton, joints: &[[i16; 3]], head: i16, upper: i16) -> Vec<Mat4> {
        skeleton.pose_override(joints, |limb, _pos, rot| {
            if limb == REDEAD_GIBDO_LIMB_HEAD_ROOT {
                rot[1] = rot[1].wrapping_add(head);
            } else if limb == REDEAD_GIBDO_LIMB_UPPER_BODY_ROOT {
                rot[1] = rot[1].wrapping_add(upper);
            }
            Mat4::IDENTITY
        })
    }

    /// Hidden by `Actor_DrawAll` for want of the Lens of Truth: `ACTOR_FLAG_REACT_TO_LENS` in a
    /// room whose lens actors show only through it (`LENS_MODE_SHOW_ACTORS`), or out of its room.
    fn lens_hidden(&self, play: &PlayState) -> bool {
        self.actor.flags & ACTOR_FLAG_REACT_TO_LENS != 0 && (play.room_ctx.cur.lens_mode == 0 || self.actor.room != play.room_ctx.cur.num)
    }
}

impl ActorImpl for EnRd {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnRd_Update`.
    fn update(&mut self, play: &mut PlayState) {
        self.update_damage(play);
        if play.save.suns_song_state != SUNSSONG_INACTIVE && !self.stunned_by_suns_song {
            play.save.suns_song_state = SUNSSONG_INACTIVE;
        }
        if self.damage_reaction != REDEAD_DMG_REACT_ICE_MAGIC && (self.action != REDEAD_ACTION_RISE_FROM_COFFIN || self.damage_reaction != REDEAD_DMG_REACT_FIRE_MAGIC) {
            if self.player_stun_wait_timer != 0 {
                self.player_stun_wait_timer -= 1;
            }
            self.run_action(play);
            if self.action != REDEAD_ACTION_GRAB && self.actor.speed_xz != 0.0 {
                // Actor_MoveXZGravity.
                self.actor.move_forward();
            }
            if self.actor.shape_rot.x == 0 && self.action != REDEAD_ACTION_GRAB && self.actor.speed_xz != 0.0 {
                let flags = UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2 | UPDBGCHECKINFO_FLAG_3 | UPDBGCHECKINFO_FLAG_4;
                self.actor.update_bg_check_info(&play.col, 30.0, 20.0, 35.0, flags);
            }
            if self.action == REDEAD_ACTION_ATTEMPT_PLAYER_FREEZE {
                self.turn_towards_player();
            }
        }
        self.actor.focus_pos = self.actor.world_pos;
        self.actor.focus_pos.y += 50.0;
        if self.actor.col_chk_info.health > 0 && self.action != REDEAD_ACTION_GRAB {
            self.collider.update(&self.actor);
            play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
            let new_swing = player_view(play).is_some_and(|p| p.unk_844 != 0 && p.unk_845 != self.unk_31d);
            if self.action != REDEAD_ACTION_DAMAGED || new_swing {
                play.collision_check_set_ac(&self.actor, 0, &mut self.collider);
            }
        }
    }

    /// `EnRd_Draw`'s changes to the actor, at full alpha: `EnRd_PostLimbDraw`'s fire positions
    /// (300 along each of ten limbs, while burning or red), then the burn: the colour kept on,
    /// a flame every 4 frames on the next body part down (`EffectSsEnFire_SpawnVec3s`, 0x4B).
    fn draw_update(&mut self, play: &mut PlayState) {
        if self.alpha != 255 || self.lens_hidden(play) {
            return;
        }
        let Some(skeleton) = self.skeleton.clone() else { return };
        if self.fire_timer != 0 || (self.actor.color_filter_timer != 0 && self.actor.color_filter_params & 0x4000 != 0) {
            let model = actor_draw_matrix(&RenderState::of(&self.actor));
            let bones = Self::pose(&skeleton, &self.skel.joint_table, self.head_y_rotation, self.upper_body_y_rotation);
            for &l in &skeleton.draw_order {
                let limb = l as usize + 1;
                let idx = match limb {
                    REDEAD_GIBDO_LIMB_HEAD => 0,
                    REDEAD_GIBDO_LIMB_ROOT => 1,
                    REDEAD_GIBDO_LIMB_RIGHT_HAND => 2,
                    REDEAD_GIBDO_LIMB_LEFT_HAND => 3,
                    REDEAD_GIBDO_LIMB_TORSO => 4,
                    REDEAD_GIBDO_LIMB_PELVIS => 5,
                    REDEAD_GIBDO_LIMB_RIGHT_SHIN => 6,
                    REDEAD_GIBDO_LIMB_LEFT_SHIN => 7,
                    REDEAD_GIBDO_LIMB_RIGHT_FOOT => 8,
                    REDEAD_GIBDO_LIMB_LEFT_FOOT => 9,
                    _ => continue,
                };
                let d = (model * bones[l as usize]).transform_point3(Vec3::new(300.0, 0.0, 0.0));
                self.fire_pos[idx] = [d.x as i32 as i16, d.y as i32 as i16, d.z as i32 as i16];
            }
        }
        if self.fire_timer != 0 {
            self.actor.color_filter_timer = self.actor.color_filter_timer.wrapping_add(1);
            self.fire_timer -= 1;
            if self.fire_timer % 4 == 0 {
                let i = (self.fire_timer >> 2) as usize;
                let p = self.fire_pos[i];
                let r = self.actor.shape_rot;
                let me = play.cur_actor.map(|h| oot_game::effect::SsActor { handle: h, world_pos: self.actor.world_pos, shape_rot: [r.x, r.y, r.z] });
                // EffectSsEnFire_SpawnVec3s: flags | 0x8000 (the body part's Vec3s).
                play.effect_ss_en_fire_spawn_vec3f(me, Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32), 0x4B, 0, 0x8000u16 as i16, i as i16);
            }
        }
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        rs.angles = vec![self.head_y_rotation, self.upper_body_y_rotation];
        rs.values = vec![self.alpha as f32];
        rs.switches = vec![self.gibdo as u32, (self.alpha != 255) as u32];
        rs
    }

    /// `EnRd_Draw`: at full alpha the opaque skeleton (env alpha 255); else the translucent one
    /// (`gActorSetupXluDL`) at its alpha; the circle shadow under it.
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let (Some(joints), Some(skeleton)) = (&rs.joints, &self.skeleton) else { return };
        let ([head, upper], [alpha], [gibdo, xlu]) = (rs.angles.as_slice(), rs.values.as_slice(), rs.switches.as_slice()) else { return };
        if self.lens_hidden(play) {
            return;
        }
        let xlu = *xlu != 0;
        let alpha = alpha.clamp(0.0, 255.0) as u8;
        let bones = Self::pose(skeleton, &joints.rot, *head, *upper);
        let mut sv = SegmentValues::default();
        sv.env[SEG_ALPHA as usize] = Some([0, 0, 0, alpha]);
        let cmd = DrawCmd {
            mesh: MeshKey::named(keys::bake(&bake_name(*gibdo != 0, xlu))),
            transform: actor_draw_matrix(rs),
            bones,
            params: DrawParams { segments: Some(sv), ..Default::default() },
        };
        if xlu {
            out.xlu.push(cmd);
        } else {
            out.opa.push(cmd);
        }
        // func_80033C30(&world.pos, sShadowScale (0.25, 0.25, 0.25), 255 opaque or alpha).
        out.opa.push(oot_game::shadow::func_80033c30(&play.col, rs.pos, Vec3::splat(0.25), alpha));
    }

    /// `EnRd_Destroy`: the Sun's Song's state cleared.
    fn destroy(&mut self, play: &mut PlayState) {
        if play.save.suns_song_state != SUNSSONG_INACTIVE {
            play.save.suns_song_state = SUNSSONG_INACTIVE;
        }
    }

    /// `firePos` (`EffectSsEnFire`, `Vec3s`): the body parts' places at 0x14C.
    fn effect_fire_pos(&self, i: usize, _vec3s: bool) -> Vec3 {
        self.fire_pos.get(i).map(|p| Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32)).unwrap_or(Vec3::ZERO)
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
