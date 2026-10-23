//! `En_Bigokuta` (`ovl_En_Bigokuta/z_en_bigokuta.c`): Big Octo, the mini-boss on Lord
//! Jabu-Jabu's rotating platform in room 6 (GAME-10 milestone 8), `object_bigokuta`'s skeleton (19
//! limbs, flex). He's no room's placement: `Bg_Bdan_Objects` (type 0, the platform) spawns him as
//! its child, and he turns it by writing its `world.rot.y` (`func_809BD2E4`: `unk_19A`, scaled
//! 0x10 a frame).
//!
//! - **Ruto's abduction** (params 0, spawned 140 over the platform as it descends with him): a
//!   prop (`ACTORCAT_PROP`) at first, `object_bigokuta_Anim_0014B8` looped, his count from 121
//!   (`func_809BD84C`): his voice at 13 and -20, the mini-boss music (`func_800F5ACC`) at 1. The
//!   platform landed, it sets his params to 1: he **jumps to its rim** (`func_809BD8DC`:
//!   `NA_SE_EN_STAL_JUMP`; 21 frames, 263 along x, an arc 200 (below 14) or 130 over 70 above his
//!   home; landing two splashes, `NA_SE_EN_DAIOCTA_LAND_WATER`, `NA_SE_EN_GOLON_LAND_BIG`, a
//!   rumble; a bob of 20 for 9 frames), then **turns** (`func_809BDAE8`: to `home.rot.y + 0x4000`,
//!   0x400 a frame scaled; 10 frames on) onto the rim: an enemy, his params 2 (the platform's cue:
//!   the battle's on), his voice, and 40 frames' wait (`func_809BDB90`).
//! - **Met before** (params 3, `INFTABLE_RUTO_ABDUCTED`: spawned at the platform's init on its
//!   rim): one frame's wait, the mini-boss music, the fight.
//! - **Circling** (`func_809BD3F8` / `func_809BDC08`, `object_bigokuta_Anim_001CA4`,
//!   `NA_SE_EN_OCTAROCK_BUBLE` each loop): he swims round the platform's rim (263 from its middle,
//!   facing along it: `func_809BCE3C`), the platform turning 0x200 against him; his speed 0x130 a
//!   frame plus `(Link's distance from the middle − 180) × 8/15`, 0x80 less while Link's off his
//!   side of the platform (or behind him, neither facing the other: then his count of 350 runs
//!   down), 0x20 more with Link within 180, a tenth more on his last health; a wake of splashes
//!   (`func_809BCF68`, `NA_SE_EN_DAIOCTA_SPLASH`). His count out, he **turns about**
//!   (`func_809BD4A4` / `func_809BDFC8`: 0x800 a frame scaled, the platform the other way, his
//!   voice at 20) and circles the other way. Link within 250 not before him for 80 frames, he
//!   **sinks** (`func_809BD768`: `NA_SE_EN_DAIOCTA_SINK`, a new way round at random, 10 a frame
//!   and spinning, to 200 down) and **rises** (`func_809BD7F0` / `func_809BE518`) on the rim
//!   across the middle from Link.
//! - **Hit**: his front is two hard cylinders (`AC_HARD`: Link's blows bounce off) that hurt 8
//!   (`AT_TYPE_ENEMY`) and knock Link away (`func_809BE798`: `Actor_SetPlayerKnockbackLargeNoDamage`,
//!   10 at his yaw ± 0x4000 or 0x6000, 5 up; circling he turns about at once, 40 frames); his back
//!   a sphere (75, 45 up and 30 behind) that takes the hits (`EnBigokuta_UpdateDamage`):
//!   - a Deku nut or the boomerang (reaction 1) **stuns** him (`func_809BD524` / `func_809BE058`:
//!     `object_bigokuta_Anim_000D1C`, `NA_SE_EN_DAIOCTA_MAHI`, blue 80 frames, a nut's 20; his
//!     head flashing (limb 15's env); pushing Link along his yaw while they touch), then he **spins**
//!     (`func_809BD6B8` / `func_809BE3E4`: 0x2000 a frame, 24 or 28 frames at random; after a
//!     nut 4 facing away from Link, or none) and circles, after a stun dizzy (40 frames of 0x200);
//!   - a bomb (reaction 0xF) throws him up (`func_809BD47C` / `func_809BDF34`: 16 frames, 100 high;
//!     landing four splashes, the landing sounds, a quake of 10 for 8 frames);
//!   - the rest (a Deku stick 2, the Kokiri Sword 1, its spin 1, its jump 2, the Master Sword's
//!     jump 4) only from behind (Link not within 0x4000 of his face): `NA_SE_EN_DAIOCTA_DAMAGE`
//!     (or `_DEAD`, the finishing blow), **hurt** (`func_809BD5E0` / `func_809BE180`:
//!     `object_bigokuta_Anim_000444`, red 24 frames, up 100 and down spinning, landing at 8, his
//!     body squashed at random every other frame); health 4.
//! - **Dying** (`func_809BD658` / `func_809BE26C`, `object_bigokuta_Anim_000A74`,
//!   `NA_SE_EN_DAIOCTA_DEAD2`): 38 frames darkening (limb 10's env) and spinning 0x2000 a frame to
//!   the last 10; the animation, then 10 more frames of limb 17 swelling; at the 4th a puff
//!   (`func_8002829C`, 150 up, 1200) and `NA_SE_EN_OCTAROCK_DEAD2`; then shrinking (`scale.y`
//!   0.001 a frame) and fading (drawn translucent): gone, the room cleared (`Flags_SetClear`), the
//!   main camera back to `CAM_SET_DUNGEON0` with the bg's settings, `NA_SE_EN_OCTAROCK_BUBLE`, a
//!   drop from table 11.
//! - Every update the main camera is asked for `CAM_SET_BIG_OCTO` without the bg's settings
//!   (`CAM_STATE_CHECK_BG` off), and ripples come up under him every 7th frame when he's at the
//!   surface or under it (`func_809BCEBC`).
//!
//! The whole overlay is ported but the rumbles (`Rumble_Request`). Its draw (`EnBigokuta_Draw`) is
//! the skeleton (`SkelAnime_DrawFlexOpa`) in three bakes split where
//! `EnBigokuta_OverrideLimbDraw`'s `gDPSetEnvColor`s fall (limbs 10 and 15), or translucent
//! (`gActorSetupXluDL`, env alpha by `scale.y`) at the end.
//!
//! Params: `params` whole: 0 Ruto's abduction (the platform's, waiting for it), 3 met before
//! (fights at once); 1 and 2 are the platform's and his own writes (the jump's cue; the battle on).
//! Flags: sets the room's clear flag as he dies.

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::math3d::{Cylinder16, Sphere16};
use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use eng_math::{cos_s, scaled_step_to_s, sin_s, step_to_f, vec3f_yaw};
use glam::{Mat4, Vec3};
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_ENEMY, ACTORCAT_PROP, ActorImpl, ActorProfile, actor_set_player_knockback_large_no_damage, audio_play_actor_sfx2, enemy_start_finishing_blow};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind};
use oot_game::audio::sfx::SFX_FLAG;
use oot_game::camera::{CAM_ID_MAIN, CAM_SET_DUNGEON0};
use oot_game::collision_check::*;
use oot_game::gbi::Dl;
use oot_game::pack::{BakeBody, BakeSegment, LimbOverride, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::skelanime_std::{ANIMMODE_LOOP, ANIMMODE_ONCE, Anim, LimbDraw, SkelAnimeStd, draw_opa_pose};

use crate::en_eiyer::is_facing_player;

/// `ACTOR_EN_BIGOKUTA` (`actor_table.h`: 0x00C6).
pub const ACTOR_EN_BIGOKUTA: i16 = 0x00C6;
/// `OBJECT_BIGOKUTA` (`object_table.h`: 0x015F).
pub const OBJECT: &str = "object_bigokuta";
pub const OBJECT_BIGOKUTA: i16 = 0x015F;
/// `object_bigokuta_Skel_006BC0` (`object_bigokuta.xml`): 19 limbs (`SkelAnime_InitFlex`'s 20
/// with the root's).
pub const SKEL: &str = "object_bigokuta_Skel_006BC0";
const LIMBS: usize = 19;

/// The animations (`object_bigokuta.xml`): floating (the intro, turning about), swimming round,
/// stunned, hurt, dying.
pub const ANIM_FLOAT: &str = "object_bigokuta_Anim_0014B8";
pub const ANIM_SWIM: &str = "object_bigokuta_Anim_001CA4";
pub const ANIM_STUN: &str = "object_bigokuta_Anim_000D1C";
pub const ANIM_HURT: &str = "object_bigokuta_Anim_000444";
pub const ANIM_DIE: &str = "object_bigokuta_Anim_000A74";

/// `En_Bigokuta_Profile`: `ACTORCAT_ENEMY`, attention-enabled, hostile, never culled,
/// `OBJECT_BIGOKUTA`.
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_EN_BIGOKUTA,
    name: "En_Bigokuta",
    category: ACTORCAT_ENEMY,
    flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE | ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_DRAW_CULLING_DISABLED,
    object: OBJECT,
    schema: &SCHEMA,
};

/// `params`, whole: how he begins, and the platform's handshake.
pub const PARAMS: Field = Field::whole(
    "params",
    Kind::Named(&[
        (0, "Ruto's abduction: on the platform, waiting for its cue"),
        (1, "the platform's cue (written by Bg_Bdan_Objects): jump to the rim"),
        (2, "on the rim (his own write): the battle's on"),
        (3, "met before (INFTABLE_RUTO_ABDUCTED): fights at once"),
    ]),
    "spawned by Bg_Bdan_Objects' platform with 0 or 3",
);

/// En_Bigokuta's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[PARAMS],
    flags: &[FlagUse::new(Flag::Clear).sets().note("as he dies (the platform sinks on it)")],
    todo: false,
    objects: &[],
    texts: &[],
};

/// The params' values (the platform's protocol: `z_bg_bdan_objects.c`).
pub const PARAMS_INTRO: i16 = 0;
pub const PARAMS_JUMP: i16 = 1;
pub const PARAMS_BATTLE: i16 = 2;
pub const PARAMS_MET: i16 = 3;

/// `CAM_SET_BIG_OCTO` (`camera.h`: 0x34, "TAKO").
pub const CAM_SET_BIG_OCTO: i16 = 0x34;
/// `CAM_STATE_CHECK_BG` (`camera.h`: 1 << 2).
pub const CAM_STATE_CHECK_BG: i16 = 1 << 2;
/// `NA_BGM_MINI_BOSS` (`sequence_table.h`: `Sequence_56`, 0x38).
pub const NA_BGM_MINI_BOSS: u16 = 0x38;
/// `NAVI_ENEMY_BIGOCTO` (`actor.h`: 0x59).
pub const NAVI_ENEMY_BIGOCTO: u8 = 0x59;
/// `ATTENTION_RANGE_2` (`actor.h`).
const ATTENTION_RANGE_2: u8 = 2;
/// `COLLECTIBLE_DROP_RANDOM_PARAMS(COLLECTIBLE_DROP_TABLE_11, false)` (`z_en_item00.h`: the table
/// × 16).
pub const DROP_TABLE_11: i16 = 11 * 16;

// The sounds (`include/tables/sfx/enemybank_table.h`).
/// `NA_SE_EN_STAL_JUMP` (0x386C).
pub const NA_SE_EN_STAL_JUMP: u16 = 0x386C;
/// `NA_SE_EN_GOLON_LAND_BIG` (0x3879).
pub const NA_SE_EN_GOLON_LAND_BIG: u16 = 0x3879;
/// `NA_SE_EN_OCTAROCK_BUBLE` (0x38C5).
pub const NA_SE_EN_OCTAROCK_BUBLE: u16 = 0x38C5;
/// `NA_SE_EN_OCTAROCK_DEAD2` (0x38C7).
pub const NA_SE_EN_OCTAROCK_DEAD2: u16 = 0x38C7;
/// `NA_SE_EN_DAIOCTA_MAHI` (0x3904).
pub const NA_SE_EN_DAIOCTA_MAHI: u16 = 0x3904;
/// `NA_SE_EN_DAIOCTA_SPLASH` (0x3905).
pub const NA_SE_EN_DAIOCTA_SPLASH: u16 = 0x3905;
/// `NA_SE_EN_DAIOCTA_VOICE` (0x3906).
pub const NA_SE_EN_DAIOCTA_VOICE: u16 = 0x3906;
/// `NA_SE_EN_DAIOCTA_DAMAGE` (0x3907).
pub const NA_SE_EN_DAIOCTA_DAMAGE: u16 = 0x3907;
/// `NA_SE_EN_DAIOCTA_SINK` (0x3908).
pub const NA_SE_EN_DAIOCTA_SINK: u16 = 0x3908;
/// `NA_SE_EN_DAIOCTA_DEAD` (0x3909).
pub const NA_SE_EN_DAIOCTA_DEAD: u16 = 0x3909;
/// `NA_SE_EN_DAIOCTA_DEAD2` (0x390A).
pub const NA_SE_EN_DAIOCTA_DEAD2: u16 = 0x390A;
/// `NA_SE_EN_DAIOCTA_LAND_WATER` (0x39ED).
pub const NA_SE_EN_DAIOCTA_LAND_WATER: u16 = 0x39ED;

/// His distance from the platform's middle on its rim.
pub const RIM: f32 = 263.0;

/// `sJntSphElementsInit`: his back, 75 across, 45 up and 30 behind his middle (limb 1, set by
/// hand in `func_809BE568`); hurting 8 (`DMG_UNBLOCKABLE`'s bit, `ATELEM_SFX_HARD`) while it
/// can't be hit, hit by Player's `0xFFCFFFFF`.
pub const JNT_SPH_ELEMENTS: [ColliderJntSphElementInit; 1] = [ColliderJntSphElementInit {
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK1,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0x2000_0000, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x08 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: HIT_BACKLASH_NONE, defense: 0x00 },
        at_elem_flags: ATELEM_ON | ATELEM_SFX_HARD,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_ON,
    },
    limb: 1,
    model_sphere: Sphere16 { center: [0, 45, -30], radius: 75 },
    scale: 100,
}];

/// `sJntSphInit`.
pub const JNT_SPH_INIT: ColliderInit = ColliderInit { col_type: COL_MATERIAL_HIT0, at_flags: AT_ON | AT_TYPE_ENEMY, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_JNTSPH };

/// `sCylinderInit`: his front, two hard cylinders 50 by 100 at (±30, 0, 12) in his frame; hurting
/// 8, hit by Player's `0xFFCFFFE7` (`AC_HARD`: they bounce off).
pub const CYLINDER_INIT: [ColliderCylinderInit; 2] = [cylinder_init(30), cylinder_init(-30)];

const fn cylinder_init(x: i16) -> ColliderCylinderInit {
    ColliderCylinderInit {
        base: ColliderInit { col_type: COL_MATERIAL_HARD, at_flags: AT_ON | AT_TYPE_ENEMY, ac_flags: AC_ON | AC_HARD | AC_TYPE_PLAYER, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_CYLINDER },
        info: ColliderElementInit {
            elem_material: ELEM_MATERIAL_UNK1,
            at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0x2000_0000, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x08 },
            ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFE7, hit_backlash: HIT_BACKLASH_NONE, defense: 0x00 },
            at_elem_flags: ATELEM_ON | ATELEM_SFX_HARD,
            ac_elem_flags: ACELEM_ON,
            oc_elem_flags: OCELEM_ON,
        },
        dim: Cylinder16 { radius: 50, height: 100, y_shift: 0, pos: [x, 0, 12] },
    }
}

/// `sColChkInfoInit`: health 4, 130 by 120, mass 200.
pub const COL_CHK_INFO_INIT: CollisionCheckInfoInit = CollisionCheckInfoInit { health: 4, cyl_radius: 130, cyl_height: 120, mass: 200 };

/// `sDamageTable`.
pub static S_DAMAGE_TABLE: DamageTable = DamageTable {
    table: [
        dmg_entry(0, 0x1), // Deku nut
        dmg_entry(2, 0x0), // Deku stick
        dmg_entry(0, 0x0), // Slingshot
        dmg_entry(0, 0xF), // Explosive
        dmg_entry(0, 0x1), // Boomerang
        dmg_entry(0, 0x0), // Normal arrow
        dmg_entry(0, 0x0), // Hammer swing
        dmg_entry(0, 0x0), // Hookshot
        dmg_entry(1, 0x0), // Kokiri sword
        dmg_entry(0, 0x0), // Master sword
        dmg_entry(0, 0x0), // Giant's Knife
        dmg_entry(0, 0x0), // Fire arrow
        dmg_entry(0, 0x0), // Ice arrow
        dmg_entry(0, 0x0), // Light arrow
        dmg_entry(0, 0x0), // Unk arrow 1
        dmg_entry(0, 0x0), // Unk arrow 2
        dmg_entry(0, 0x0), // Unk arrow 3
        dmg_entry(0, 0x0), // Fire magic
        dmg_entry(0, 0x0), // Ice magic
        dmg_entry(0, 0x0), // Light magic
        dmg_entry(0, 0x0), // Shield
        dmg_entry(0, 0x0), // Mirror Ray
        dmg_entry(1, 0x0), // Kokiri spin
        dmg_entry(0, 0x0), // Giant spin
        dmg_entry(0, 0x0), // Master spin
        dmg_entry(2, 0x0), // Kokiri jump
        dmg_entry(0, 0x0), // Giant jump
        dmg_entry(4, 0x0), // Master jump
        dmg_entry(0, 0x0), // Unknown 1
        dmg_entry(0, 0x0), // Unblockable
        dmg_entry(0, 0x0), // Hammer jump
        dmg_entry(0, 0x0), // Unknown 2
    ],
};

/// `sEffectPrimColor`, `sEffectEnvColor`: the dying puff's colours.
const EFFECT_PRIM: [u8; 4] = [255, 255, 255, 255];
const EFFECT_ENV: [u8; 4] = [100, 255, 255, 255];

/// The limbs `EnBigokuta_OverrideLimbDraw` acts on (1-based): limb 10's env (the body's colour,
/// darkening as he dies), limb 15's (the head's flash, stunned or hurt), limb 17 swelling as he
/// dies.
pub const LIMB_BODY_ENV: usize = 10;
pub const LIMB_HEAD_ENV: usize = 15;
pub const LIMB_SWELL: usize = 17;

/// The bakes: the skeleton split where the override's env colours fall (in its draw order: limbs
/// 1-9 under the draw's white, 10-15 under limb 10's, 16-19 under limb 15's or limb 10's), and
/// the whole of it translucent.
pub const BAKE_FRONT: &str = "En_Bigokuta/opa_1_9";
pub const BAKE_BODY: &str = "En_Bigokuta/opa_10_15";
pub const BAKE_BACK: &str = "En_Bigokuta/opa_16_19";
pub const BAKE_XLU: &str = "En_Bigokuta/xlu";
/// Segment 0xC: `ACTOR_SETUP_OPA_DL` (an end) or `gActorSetupXluDL`; every limb's list calls it
/// after its render mode.
const SEG_SETUP: u8 = 0x0C;
/// The env colour the draw and the override set, a dynamic colour before the limbs.
const SEG_ENV: u8 = 0x0E;

// shared: gActorSetupXluDL (z_actor.c), as demo_du.rs and demo_sa.rs build it.
/// `gActorSetupXluDL`: `gsDPSetRenderMode(G_RM_FOG_SHADE_A, G_RM_AA_ZB_XLU_SURF2)`,
/// `gsDPSetAlphaCompare(G_AC_THRESHOLD)`, the end.
fn actor_setup_xlu_dl() -> Vec<(u32, u32)> {
    let mut d = Dl::default();
    d.0.push((0xE200_001C, 0xC810_49F8));
    d.othermode_l(0, 2, 1);
    d.end();
    d.0
}

fn skel_bake(name: &str, keep: std::ops::RangeInclusive<usize>, setup: Vec<(u32, u32)>) -> MeshBake {
    MeshBake {
        name: name.into(),
        object: OBJECT.into(),
        segments: vec![(SEG_SETUP, BakeSegment::Commands(setup)), (SEG_ENV, BakeSegment::DynamicColor { env: true, prim: false })],
        prelude: vec![SEG_ENV],
        body: BakeBody::Skeleton {
            file: OBJECT.into(),
            symbol: SKEL.into(),
            limbs: (1..=LIMBS).filter(|l| !keep.contains(l)).map(|l| LimbOverride { limb: (l - 1) as u8, file: OBJECT.into(), symbol: String::new() }).collect(),
        },
    }
}

/// The meshes `EnBigokuta_Draw` draws (after `Gfx_SetupDL_25Opa` / `_25Xlu`).
pub fn bakes() -> Vec<MeshBake> {
    vec![
        skel_bake(BAKE_FRONT, 1..=9, Vec::new()),
        skel_bake(BAKE_BODY, 10..=15, Vec::new()),
        skel_bake(BAKE_BACK, 16..=19, Vec::new()),
        skel_bake(BAKE_XLU, 1..=19, actor_setup_xlu_dl()),
    ]
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `func_809BD84C`: on the platform, waiting for its cue (params 1).
    IntroWait,
    /// `func_809BD8DC`: the jump to the rim.
    IntroJump,
    /// `func_809BDAE8`: turning onto the rim.
    IntroTurn,
    /// `func_809BDB90`: the wait before the fight.
    WaitToFight,
    /// `func_809BDC08`: swimming round the rim.
    Circle,
    /// `func_809BDF34`: thrown up by a bomb.
    Bounce,
    /// `func_809BDFC8`: turning about.
    TurnAround,
    /// `func_809BE058`: stunned.
    Stunned,
    /// `func_809BE180`: hurt.
    Hurt,
    /// `func_809BE26C`: dying.
    Die,
    /// `func_809BE3E4`: spinning out of a stun.
    Spin,
    /// `func_809BE4A4`: sinking.
    Sink,
    /// `func_809BE518`: rising.
    Rise,
}

impl Action {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Action::IntroWait => "func_809BD84C",
            Action::IntroJump => "func_809BD8DC",
            Action::IntroTurn => "func_809BDAE8",
            Action::WaitToFight => "func_809BDB90",
            Action::Circle => "func_809BDC08",
            Action::Bounce => "func_809BDF34",
            Action::TurnAround => "func_809BDFC8",
            Action::Stunned => "func_809BE058",
            Action::Hurt => "func_809BE180",
            Action::Die => "func_809BE26C",
            Action::Spin => "func_809BE3E4",
            Action::Sink => "func_809BE4A4",
            Action::Rise => "func_809BE518",
        }
    }
}

struct Anims {
    float: Anim,
    swim: Anim,
    stun: Anim,
    hurt: Anim,
    die: Anim,
}

impl Anims {
    fn load(play: &PlayState) -> Option<Anims> {
        let a = play.assets.clone()?;
        let get = |s: &str| a.animation(OBJECT, s).map_err(|e| log::error!("En_Bigokuta: {e:#}")).ok();
        Some(Anims { float: get(ANIM_FLOAT)?, swim: get(ANIM_SWIM)?, stun: get(ANIM_STUN)?, hurt: get(ANIM_HURT)?, die: get(ANIM_DIE)? })
    }
}

pub struct EnBigokuta {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    skeleton: Option<Arc<Skeleton>>,
    anims: Option<Anims>,
    pub action: Action,
    /// `unk_194`: his way round the rim (1 or -1).
    pub unk_194: i8,
    /// `unk_195`: a short spin out of the stun next (a Deku nut's; turning about sets it, and
    /// circling it lets him turn about once at random with Link at his side).
    pub unk_195: bool,
    /// `unk_196`: the actions' count.
    pub unk_196: i16,
    /// `unk_198`: circling, the frames left before he sinks (Link near, not before him); dying,
    /// the frames after the animation.
    pub unk_198: i16,
    /// `unk_19A`: the platform's turn a frame (its `world.rot.y`, stepped to this).
    pub unk_19a: i16,
    /// `collider`: his back.
    pub collider: ColliderJntSph,
    /// `colliderCylinders`: his front.
    pub collider_cylinders: [ColliderCylinder; 2],
    /// `EnBigokuta_Draw`'s random squash this frame (`Rand_ZeroOne` × 2π about x, then y), drawn
    /// hurt or dying on odd counts.
    pub squash: Option<(f32, f32)>,
}

impl EnBigokuta {
    /// `EnBigokuta_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // sInitChain.
        actor.target_arrow_offset = 2000.0;
        actor.target_mode = ATTENTION_RANGE_2;
        actor.gravity = -1.0;
        actor.navi_enemy_id = NAVI_ENEMY_BIGOCTO;
        actor.scale = Vec3::splat(0.033);
        let anims = Anims::load(play);
        let skeleton = play.assets.clone().and_then(|a| a.skeleton(OBJECT, SKEL).map_err(|e| log::error!("En_Bigokuta: {e:#}")).ok());
        // SkelAnime_InitFlex(.., &object_bigokuta_Anim_0014B8, .., 20).
        let skel = SkelAnimeStd::init_flex(skeleton.as_ref().map(|s| s.limbs.len()).unwrap_or(LIMBS), anims.as_ref().map(|a| a.float.clone()));
        let mut collider = ColliderJntSph::new(&JNT_SPH_INIT, &JNT_SPH_ELEMENTS);
        collider.elements[0].dim.world_sphere.radius = collider.elements[0].dim.model_sphere.radius;
        let mut this = EnBigokuta {
            actor,
            skel,
            skeleton,
            anims,
            action: Action::IntroWait,
            unk_194: 1,
            unk_195: false,
            unk_196: 0,
            unk_198: 0,
            unk_19a: 0,
            collider,
            collider_cylinders: [ColliderCylinder::new(&CYLINDER_INIT[0]), ColliderCylinder::new(&CYLINDER_INIT[1])],
            squash: None,
        };
        this.actor.col_chk_info.set_info(Some(&S_DAMAGE_TABLE), &COL_CHK_INFO_INIT);
        if PARAMS.get(this.actor.params) == PARAMS_INTRO {
            // Actor_ChangeCategory(.., ACTORCAT_PROP): the arena adds it to the category the init
            // leaves.
            this.actor.category = ACTORCAT_PROP;
            this.setup_intro_wait();
        } else {
            this.setup_wait_to_fight();
            this.unk_19a = 0;
            this.unk_196 = 1;
            this.actor.home_pos.y = -1025.0;
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

    /// `Animation_MorphToLoop`.
    fn morph_to_loop(&mut self, f: impl Fn(&Anims) -> &Anim, morph_frames: f32) {
        if let Some(a) = self.anim(f) {
            let last = a.last_frame();
            self.skel.change(a, 1.0, 0.0, last, ANIMMODE_LOOP, morph_frames);
        }
    }

    /// `Animation_MorphToPlayOnce`.
    fn morph_to_play_once(&mut self, f: impl Fn(&Anims) -> &Anim, morph_frames: f32) {
        if let Some(a) = self.anim(f) {
            let last = a.last_frame();
            self.skel.change(a, 1.0, 0.0, last, ANIMMODE_ONCE, morph_frames);
        }
    }

    fn player(play: &PlayState) -> Option<&Actor> {
        play.player.and_then(|h| play.actors.actor(h))
    }

    /// `func_809BCE3C`: on the rim, facing along it (`world.rot.y` a quarter turn from his face,
    /// against his way round), 263 from the middle (`home.pos`).
    fn set_on_rim(&mut self) {
        let a = &mut self.actor;
        a.world_rot.y = (a.shape_rot.y as i32 + self.unk_194 as i32 * -0x4000) as i16;
        a.world_pos.x = (sin_s(a.world_rot.y) * RIM) + a.home_pos.x;
        a.world_pos.z = (cos_s(a.world_rot.y) * RIM) + a.home_pos.z;
    }

    /// `func_809BCEBC`: every 7th frame a ripple under him at the surface (3 over his home), when
    /// he's at it or less than 100 under it.
    fn spawn_ripple(&self, play: &mut PlayState) {
        let y_dist_from_home = self.actor.world_pos.y - self.actor.home_pos.y;
        let pos = Vec3::new(self.actor.world_pos.x, self.actor.home_pos.y + 3.0, self.actor.world_pos.z);
        if play.gameplay_frames % 7 == 0 && y_dist_from_home <= 0.0 && y_dist_from_home > -100.0 {
            play.with_ss(|s| s.g_ripple_spawn(pos, 800, 1300, 0));
        }
    }

    /// `func_809BCF68`: his wake: a splash behind him (a random angle 0x1200 ± 0x600 off his back
    /// against his way round): 80 back with a ripple spinning or on frames with bit 1, else 120 back
    /// (5 up); sinking 50 back with a ripple; `NA_SE_EN_DAIOCTA_SPLASH` but sinking.
    fn spawn_wake(&mut self, play: &mut PlayState) {
        let base = self.actor.shape_rot.y as i32 - self.unk_194 as i32 * 0xA00;
        let rot = if play.gameplay_frames & 1 != 0 { (play.rand.s16_offset(0x1200, 0xC00) as i32 + base) as i16 } else { (base - play.rand.s16_offset(0x1200, 0xC00) as i32) as i16 };
        let w = self.actor.world_pos;
        let home_y = self.actor.home_pos.y;
        let pos;
        if self.action != Action::Sink {
            if self.action == Action::Spin || play.gameplay_frames & 2 != 0 {
                pos = Vec3::new(w.x - sin_s(rot) * 80.0, home_y + 1.0, w.z - cos_s(rot) * 80.0);
                play.with_ss(|s| s.g_ripple_spawn(pos, 100, 500, 0));
            } else {
                pos = Vec3::new(w.x - sin_s(rot) * 120.0, home_y + 5.0, w.z - cos_s(rot) * 120.0);
            }
        } else {
            pos = Vec3::new(w.x - sin_s(rot) * 50.0, home_y + 1.0, w.z - cos_s(rot) * 50.0);
            play.with_ss(|s| s.g_ripple_spawn(pos, 100, 500, 0));
        }
        play.with_ss(|s| s.g_splash_spawn(pos, None, None, 1, 800));
        if self.action != Action::Sink {
            self.actor.play_sfx_flagged(NA_SE_EN_DAIOCTA_SPLASH - SFX_FLAG);
        }
    }

    /// `func_809BD1C8`: his landing: four splashes 60 out on the diagonals (2000), the landing's
    /// sounds, a quake of 10 for 8 frames.
    fn land(&mut self, play: &mut PlayState) {
        let w = self.actor.world_pos;
        for i in 0..4 {
            let x = (if i >= 2 { 1.0 } else { -1.0 }) * 60.0 + w.x;
            let z = (if i & 1 != 0 { 1.0 } else { -1.0 }) * 60.0 + w.z;
            let pos = Vec3::new(x, w.y, z);
            play.with_ss(|s| s.g_splash_spawn(pos, None, None, 1, 2000));
        }
        audio_play_actor_sfx2(play, NA_SE_EN_DAIOCTA_LAND_WATER);
        audio_play_actor_sfx2(play, NA_SE_EN_GOLON_LAND_BIG);
        play.actor_request_quake_and_rumble(10, 8);
    }

    /// `func_809BD2E4`: the platform's (`parent`'s) `world.rot.y` stepped to `unk_19A`, 0x10 a
    /// frame (scaled).
    fn turn_platform(&self, play: &mut PlayState) {
        if let Some(p) = self.actor.parent.and_then(|h| play.actors.actor_mut(h)) {
            scaled_step_to_s(&mut p.world_rot.y, self.unk_19a, 0x10);
        }
    }

    /// `func_809BD318`: floating, his home's height the surface's (-1025), 121 frames.
    fn setup_intro_wait(&mut self) {
        self.play_loop(|a| &a.float);
        self.unk_19a = 0;
        self.actor.home_pos.y = -1025.0;
        self.unk_196 = 121;
        self.action = Action::IntroWait;
    }

    /// `func_809BD370`: the jump, 21 frames, `NA_SE_EN_STAL_JUMP`.
    fn setup_intro_jump(&mut self, play: &mut PlayState) {
        self.unk_196 = 21;
        audio_play_actor_sfx2(play, NA_SE_EN_STAL_JUMP);
        self.action = Action::IntroJump;
    }

    /// `func_809BD3AC`: on the rim (263 along x) at the surface, turning, 10 frames to wait.
    fn setup_intro_turn(&mut self) {
        self.actor.world_pos.x = self.actor.home_pos.x + RIM;
        self.unk_196 = 10;
        self.action = Action::IntroTurn;
        self.actor.world_pos.y = self.actor.home_pos.y;
    }

    /// `func_809BD3E0`: 40 frames before the fight.
    fn setup_wait_to_fight(&mut self) {
        self.unk_196 = 40;
        self.action = Action::WaitToFight;
    }

    /// `func_809BD3F8`: swimming round (`object_bigokuta_Anim_001CA4`), 350 frames to turning
    /// about, 80 to sinking, the platform 0x200 against him; his front hurting, his back hittable.
    fn setup_circle(&mut self) {
        self.morph_to_loop(|a| &a.swim, -5.0);
        self.unk_196 = 350;
        self.unk_198 = 80;
        self.unk_19a = (self.unk_194 as i32 * -0x200) as i16;
        self.set_on_rim();
        self.collider_cylinders[0].base.at_flags |= AT_ON;
        self.collider.base.ac_flags |= AC_ON;
        self.action = Action::Circle;
    }

    /// `func_809BD47C`: thrown up, 16 frames; not hittable, the colour filter cut.
    fn setup_bounce(&mut self) {
        self.unk_196 = 16;
        self.collider.base.ac_flags &= !AC_ON;
        self.actor.color_filter_timer = 0;
        self.action = Action::Bounce;
    }

    /// `func_809BD4A4`: turning about (to his back's yaw, kept in `world.rot.x`), the platform
    /// 0x200 his way; not hittable, his front hurting.
    fn setup_turn_around(&mut self) {
        self.morph_to_loop(|a| &a.float, -5.0);
        self.unk_195 = true;
        self.actor.world_rot.x = self.actor.shape_rot.y.wrapping_add(-0x8000i16);
        self.unk_19a = (self.unk_194 as i32 * 0x200) as i16;
        self.collider.base.ac_flags &= !AC_ON;
        self.collider_cylinders[0].base.at_flags |= AT_ON;
        self.action = Action::TurnAround;
    }

    /// `func_809BD524`: stunned (`object_bigokuta_Anim_000D1C`, `NA_SE_EN_DAIOCTA_MAHI`), the
    /// platform still; by a Deku nut (the hit's `DMG_DEKU_NUT`) 20 frames and a short spin after,
    /// else 80; blue for as long.
    fn setup_stunned(&mut self, play: &mut PlayState) {
        self.morph_to_play_once(|a| &a.stun, -5.0);
        self.unk_196 = 80;
        self.unk_19a = 0;
        self.collider_cylinders[0].base.at_flags |= AT_ON;
        audio_play_actor_sfx2(play, NA_SE_EN_DAIOCTA_MAHI);
        let dmg_flags = match self.collider.elements[0].info.ac_hit_elem {
            Some(h) => h.at_dmg_info.dmg_flags,
            None => {
                log::warn!("En_Bigokuta: stunned with no acHitElem");
                0
            }
        };
        if dmg_flags & DMG_DEKU_NUT != 0 {
            self.unk_195 = true;
            self.unk_196 = 20;
        } else {
            self.unk_195 = false;
            self.unk_196 = 80;
        }
        self.actor.set_color_filter(COLORFILTER_COLORFLAG_BLUE, 255, COLORFILTER_BUFFLAG_OPA, self.unk_196 as u16);
        self.action = Action::Stunned;
    }

    /// `func_809BD5E0`: hurt (`object_bigokuta_Anim_000444`), 24 frames, red as long; the
    /// platform still, his front harmless.
    fn setup_hurt(&mut self) {
        self.morph_to_play_once(|a| &a.hurt, -5.0);
        self.unk_196 = 24;
        self.unk_19a = 0;
        self.collider_cylinders[0].base.at_flags &= !AT_ON;
        self.actor.set_color_filter(COLORFILTER_COLORFLAG_RED, 255, COLORFILTER_BUFFLAG_OPA, 24);
        self.action = Action::Hurt;
    }

    /// `func_809BD658`: dying (`object_bigokuta_Anim_000A74`, `NA_SE_EN_DAIOCTA_DEAD2`), 38
    /// frames, then 10.
    fn setup_die(&mut self, play: &mut PlayState) {
        self.morph_to_play_once(|a| &a.die, -5.0);
        audio_play_actor_sfx2(play, NA_SE_EN_DAIOCTA_DEAD2);
        self.unk_196 = 38;
        self.unk_198 = 10;
        self.action = Action::Die;
    }

    /// `func_809BD6B8`: spinning: 24 or 28 frames at random; after a nut 4 with Link off his face
    /// (by 0x4000), else none. Not hittable, the platform still.
    ///
    /// @bug (game): `ABS(shape.rot.y - yawTowardsPlayer)` is the two `s16`s' difference as an
    /// `int`, not wrapped: yaws either side of 0x8000 (close as they are) count as far apart.
    fn setup_spin(&mut self, play: &mut PlayState) {
        if !self.unk_195 {
            self.unk_196 = if play.rand.zero_one() < 0.5 { 24 } else { 28 };
        } else if (self.actor.shape_rot.y as i32 - self.actor.yaw_towards_player as i32).abs() >= 0x4000 {
            self.unk_196 = 4;
        } else {
            self.unk_196 = 0;
        }
        self.unk_19a = 0;
        self.collider.base.ac_flags &= !AC_ON;
        self.action = Action::Spin;
    }

    /// `func_809BD768`: sinking, a new way round at random; no lock-on, his front harmless,
    /// `NA_SE_EN_DAIOCTA_SINK`.
    fn setup_sink(&mut self, play: &mut PlayState) {
        self.unk_194 = if play.rand.zero_one() < 0.5 { -1 } else { 1 };
        self.unk_19a = 0;
        self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        self.collider_cylinders[0].base.at_flags &= !AT_ON;
        audio_play_actor_sfx2(play, NA_SE_EN_DAIOCTA_SINK);
        self.action = Action::Sink;
    }

    /// `func_809BD7F0`: on the rim across the middle from Link (the yaw from Link to the middle),
    /// facing along it his way round; rising.
    fn setup_rise(&mut self, play: &PlayState) {
        let player_pos = Self::player(play).map(|p| p.world_pos).unwrap_or_default();
        // Actor_WorldYawTowardPoint(&player->actor, &home.pos).
        self.actor.world_rot.y = vec3f_yaw(player_pos, self.actor.home_pos);
        self.actor.shape_rot.y = (self.actor.world_rot.y as i32 + (self.unk_194 as i32 * 0x4000)) as i16;
        self.set_on_rim();
        self.action = Action::Rise;
    }

    /// `func_809BD84C`: his count down; his voice at 13 and -20, the mini-boss music at 1; the
    /// platform's cue (params 1): the jump.
    fn intro_wait(&mut self, play: &mut PlayState) {
        self.skel.update();
        self.unk_196 = self.unk_196.wrapping_sub(1);
        if self.unk_196 == 13 || self.unk_196 == -20 {
            audio_play_actor_sfx2(play, NA_SE_EN_DAIOCTA_VOICE);
        }
        if self.unk_196 == 1 {
            play.audio.func_800f5acc(NA_BGM_MINI_BOSS);
        }
        if PARAMS.get(self.actor.params) == PARAMS_JUMP {
            self.setup_intro_jump(play);
        }
    }

    /// `func_809BD8DC`: 21 frames out to 263 along x (263/21 a frame), up `sin` (200 over home
    /// below frame 14, else 130 over 70 above); at 0 two splashes (40 either side, 70 back), the
    /// landing's sounds; then from -2 a bob of 20 down to -10.
    fn intro_jump(&mut self, play: &mut PlayState) {
        use std::f32::consts::PI;
        self.unk_196 = self.unk_196.wrapping_sub(1);
        if self.unk_196 >= 0 {
            step_to_f(&mut self.actor.world_pos.x, self.actor.home_pos.x + RIM, RIM / 21.0);
            if self.unk_196 < 14 {
                // sinf(unk_196 * (M_PI / 28)): M_PI a float.
                self.actor.world_pos.y = ((self.unk_196 as f32 * (PI / 28.0)).sin() * 200.0) + self.actor.home_pos.y;
            } else {
                self.actor.world_pos.y = ((((self.unk_196 - 7) as f32) * (PI / 14.0)).sin() * 130.0) + (self.actor.home_pos.y + 70.0);
            }
            if self.unk_196 == 0 {
                let w = self.actor.world_pos;
                let pos = Vec3::new(w.x + 40.0, w.y, w.z - 70.0);
                play.with_ss(|s| s.g_splash_spawn(pos, None, None, 1, 2000));
                let pos = Vec3::new(w.x - 40.0, w.y, w.z - 70.0);
                play.with_ss(|s| s.g_splash_spawn(pos, None, None, 1, 2000));
                audio_play_actor_sfx2(play, NA_SE_EN_DAIOCTA_LAND_WATER);
                audio_play_actor_sfx2(play, NA_SE_EN_GOLON_LAND_BIG);
                // not ported: Rumble_Request(0.0f, 255, 20, 150).
            }
        } else if self.unk_196 < -1 {
            self.actor.world_pos.y = self.actor.home_pos.y - ((((self.unk_196 + 1) as f32) * (PI / 10.0)).sin() * 20.0);
            if self.unk_196 == -10 {
                self.setup_intro_turn();
            }
        }
    }

    /// `func_809BDAE8`: turning to `home.rot.y + 0x4000` (0x400 a frame, scaled); there 10 frames
    /// on, on the rim at the surface (his home's height his), an enemy, params 2, his voice; 40
    /// frames' wait.
    fn intro_turn(&mut self, play: &mut PlayState) {
        if scaled_step_to_s(&mut self.actor.shape_rot.y, self.actor.home_rot.y.wrapping_add(0x4000), 0x400) {
            if self.unk_196 != 0 {
                self.unk_196 -= 1;
            }
            if self.unk_196 == 0 {
                self.set_on_rim();
                self.actor.home_pos.y = self.actor.world_pos.y;
                crate::obj_roomtimer::actor_change_category(play, &mut self.actor, ACTORCAT_ENEMY);
                // schema: params written whole (the platform's cue: the battle's on).
                self.actor.params = PARAMS_BATTLE;
                audio_play_actor_sfx2(play, NA_SE_EN_DAIOCTA_VOICE);
                self.setup_wait_to_fight();
            }
        }
    }

    /// `func_809BDB90`: the count out (params 3: the mini-boss music), circling.
    fn wait_to_fight(&mut self, play: &mut PlayState) {
        self.skel.update();
        if self.unk_196 != 0 {
            self.unk_196 -= 1;
        }
        if self.unk_196 == 0 {
            if PARAMS.get(self.actor.params) == PARAMS_MET {
                play.audio.func_800f5acc(NA_BGM_MINI_BOSS);
            }
            self.setup_circle();
        }
    }

    /// `func_809BDC08`: see the module's docs.
    fn circle(&mut self, play: &mut PlayState) {
        self.skel.update();
        if self.skel.on_frame(0.0) {
            audio_play_actor_sfx2(play, NA_SE_EN_OCTAROCK_BUBLE);
        }
        if self.unk_196 < 0 {
            // Dizzy after a stun: 0x200 a frame his way, the count up to 0 (then 350).
            self.actor.shape_rot.y = (self.actor.shape_rot.y as i32 + self.unk_194 as i32 * 0x200) as i16;
            self.set_on_rim();
            self.unk_196 += 1;
            if self.unk_196 == 0 {
                self.unk_196 = 350;
            }
            self.spawn_wake(play);
            return;
        }
        let (player_pos, player_yaw) = Self::player(play).map(|p| (p.world_pos, p.shape_rot.y)).unwrap_or_default();
        // Actor_WorldDistXZToPoint(&player->actor, &home.pos).
        let (dx, dz) = (self.actor.home_pos.x - player_pos.x, self.actor.home_pos.z - player_pos.z);
        let dist = ((dx * dx) + (dz * dz)).sqrt();
        let mut phi_v1 = ((dist - 180.0) * (8.0f32 / 15.0)) as i32 as i16;
        let local = self.actor.world_to_actor_coords(player_pos);
        if local.x.abs() > RIM || (local.z > 0.0 && !is_facing_player(&self.actor, 0x1B00) && !self.actor.player_is_facing(player_yaw, 0x2000)) {
            phi_v1 = phi_v1.wrapping_sub(0x80);
            if self.unk_196 != 0 {
                self.unk_196 -= 1;
            }
        }
        if self.actor.xz_dist_to_player < 250.0 && !is_facing_player(&self.actor, 0x6000) {
            if self.unk_198 != 0 {
                self.unk_198 -= 1;
            }
            if self.actor.xz_dist_to_player < 180.0 {
                phi_v1 = phi_v1.wrapping_add(0x20);
            }
        } else {
            self.unk_198 = 80;
        }
        if self.actor.col_chk_info.health == 1 {
            phi_v1 = (((phi_v1 as i32 + 0x130) as f32) * 1.1) as i32 as i16;
        } else {
            phi_v1 = phi_v1.wrapping_add(0x130);
        }
        self.actor.shape_rot.y = (self.actor.shape_rot.y as i32 + phi_v1 as i32 * self.unk_194 as i32) as i16;
        self.set_on_rim();
        self.spawn_wake(play);
        if self.unk_198 == 0 {
            self.setup_sink(play);
        } else if self.unk_196 == 0 {
            self.setup_turn_around();
        } else if self.unk_195 {
            let mut phi_v0 = self.actor.yaw_towards_player.wrapping_sub(self.actor.shape_rot.y);
            if phi_v0 < 0 {
                phi_v0 = phi_v0.wrapping_neg();
            }
            if phi_v0 < 0x4100 && phi_v0 > 0x3F00 {
                if play.rand.zero_one() < 0.6 {
                    self.unk_196 = 0;
                    self.setup_turn_around();
                }
                self.unk_195 = false;
            }
        }
    }

    /// `func_809BDF34`: up `sin(count × π/16) × 100` over the surface; down, the landing, circling.
    fn bounce(&mut self, play: &mut PlayState) {
        self.skel.update();
        if self.unk_196 != 0 {
            self.unk_196 -= 1;
        }
        self.actor.world_pos.y = ((self.unk_196 as f32 * (std::f32::consts::PI / 16.0)).sin() * 100.0) + self.actor.home_pos.y;
        if self.unk_196 == 0 {
            self.land(play);
            self.setup_circle();
        }
    }

    /// `func_809BDFC8`: the count out (his voice at 20), turned to his back's yaw (0x800 a frame,
    /// scaled): the other way round, circling.
    fn turn_around(&mut self, play: &mut PlayState) {
        self.skel.update();
        if self.unk_196 != 0 {
            self.unk_196 -= 1;
        }
        if self.unk_196 == 20 {
            audio_play_actor_sfx2(play, NA_SE_EN_DAIOCTA_VOICE);
        }
        if self.unk_196 == 0 && scaled_step_to_s(&mut self.actor.shape_rot.y, self.actor.world_rot.x, 0x800) {
            self.unk_194 = -self.unk_194;
            self.setup_circle();
        }
    }

    /// `func_809BE058`: the count out, spinning; touching Link (his OC), Link pushed along his yaw
    /// by Link's speed (at least 1): away by the back, towards him by the front.
    fn stunned(&mut self, play: &mut PlayState) {
        if self.unk_196 != 0 {
            self.unk_196 -= 1;
        }
        self.skel.update();
        if self.collider.base.oc_flags1 & OC1_HIT != 0 || self.collider_cylinders[0].base.oc_flags1 & OC1_HIT != 0 || self.collider_cylinders[1].base.oc_flags1 & OC1_HIT != 0 {
            let player = play.player;
            if let Some(p) = player.and_then(|h| play.actors.actor_mut(h)) {
                // CLAMP_MIN(player->actor.speed, 1.0f).
                let mut speed_xz = p.speed_xz.max(1.0);
                if self.collider.base.oc_flags1 & OC1_HIT == 0 {
                    self.collider_cylinders[0].base.oc_flags1 &= !OC1_HIT;
                    self.collider_cylinders[1].base.oc_flags1 &= !OC1_HIT;
                    speed_xz *= -1.0;
                }
                p.world_pos.x -= speed_xz * sin_s(self.actor.shape_rot.y);
                p.world_pos.z -= speed_xz * cos_s(self.actor.shape_rot.y);
            }
        }
        if self.unk_196 == 0 {
            self.setup_spin(play);
        }
    }

    /// `func_809BE180`: the count out: turning about, or dying with no health; down to 8 up
    /// `sin((count − 8) × π/16) × 100` spinning 0x200 a frame his way, landing at 8.
    fn hurt(&mut self, play: &mut PlayState) {
        self.skel.update();
        if self.unk_196 != 0 {
            self.unk_196 -= 1;
        }
        if self.unk_196 == 0 {
            if self.actor.col_chk_info.health != 0 {
                self.setup_turn_around();
            } else {
                self.setup_die(play);
            }
        } else if self.unk_196 >= 8 {
            self.actor.shape_rot.y = (self.actor.shape_rot.y as i32 + self.unk_194 as i32 * 0x200) as i16;
            self.actor.world_pos.y = (((self.unk_196 - 8) as f32 * (std::f32::consts::PI / 16.0)).sin() * 100.0) + self.actor.home_pos.y;
            self.set_on_rim();
            if self.unk_196 == 8 {
                self.land(play);
            }
        }
    }

    /// `func_809BE26C`: see the module's docs.
    fn die(&mut self, play: &mut PlayState) {
        if self.unk_196 != 0 {
            if self.unk_196 != 0 {
                self.unk_196 -= 1;
            }
            if self.unk_196 >= 10 {
                self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(0x2000);
            }
        } else if self.skel.update() {
            if self.unk_198 != 0 {
                self.unk_198 -= 1;
            }
            if self.unk_198 == 6 {
                let w = self.actor.world_pos;
                let pos = Vec3::new(w.x, w.y + 150.0, w.z);
                play.with_ss(|s| s.func_8002829c(pos, Vec3::ZERO, Vec3::ZERO, EFFECT_PRIM, EFFECT_ENV, 1200, 20));
                audio_play_actor_sfx2(play, NA_SE_EN_OCTAROCK_DEAD2);
            }
            if self.unk_198 == 0 && step_to_f(&mut self.actor.scale.y, 0.0, 0.001) {
                let room = self.actor.room;
                play.flags.set_clear(room);
                play.camera_change_setting(CAM_ID_MAIN, CAM_SET_DUNGEON0);
                if let Some(c) = play.camera_mut(CAM_ID_MAIN) {
                    c.state_flags |= CAM_STATE_CHECK_BG;
                }
                let pos = self.actor.world_pos;
                play.sfx_source_play_sfx_at_fixed_world_pos(pos, 50, NA_SE_EN_OCTAROCK_BUBLE);
                crate::en_item00::item_drop_collectible_random(play, Some(&self.actor), pos, DROP_TABLE_11);
                self.actor.kill();
            }
        }
    }

    /// `func_809BE3E4`: spinning 0x2000 a frame for the count; out, his way round the way he
    /// faces off his course, circling (dizzy 40 frames but after a nut); his wake.
    fn spin(&mut self, play: &mut PlayState) {
        self.skel.update();
        if self.unk_196 != 0 {
            if self.unk_196 != 0 {
                self.unk_196 -= 1;
            }
            self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(0x2000);
        }
        if self.unk_196 == 0 {
            self.unk_194 = if self.actor.shape_rot.y.wrapping_sub(self.actor.world_rot.y) > 0 { 1 } else { -1 };
            self.setup_circle();
            if !self.unk_195 {
                self.unk_196 = -40;
            }
        }
        self.spawn_wake(play);
    }

    /// `func_809BE4A4`: down 10 a frame spinning 0x2000; 200 under, rising across from Link.
    fn sink(&mut self, play: &mut PlayState) {
        self.actor.world_pos.y -= 10.0;
        self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(0x2000);
        if self.actor.world_pos.y < (self.actor.home_pos.y + -200.0) {
            self.setup_rise(play);
        }
        self.spawn_wake(play);
    }

    /// `func_809BE518`: up 10 a frame to the surface: lock-on back, circling.
    fn rise(&mut self) {
        if step_to_f(&mut self.actor.world_pos.y, self.actor.home_pos.y, 10.0) {
            self.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED;
            self.setup_circle();
        }
    }

    /// `func_809BE568`: his colliders turned with him: the sphere's centre and the cylinders'
    /// places from their model offsets (`s16`).
    fn update_colliders(&mut self) {
        let sin = sin_s(self.actor.shape_rot.y);
        let cos = cos_s(self.actor.shape_rot.y);
        let w = self.actor.world_pos;
        let e = &mut self.collider.elements[0].dim;
        let m = e.model_sphere.center.map(|c| c as f32);
        e.world_sphere.center[0] = (w.x + (m[0] * cos) + (m[2] * sin)) as i32 as i16;
        e.world_sphere.center[2] = (w.z + (m[2] * cos) - (m[0] * sin)) as i32 as i16;
        e.world_sphere.center[1] = (w.y + m[1]) as i32 as i16;
        for (c, init) in self.collider_cylinders.iter_mut().zip(CYLINDER_INIT.iter()) {
            let p = init.dim.pos.map(|v| v as f32);
            c.dim.pos[0] = (w.x + p[2] * sin + p[0] * cos) as i32 as i16;
            c.dim.pos[2] = (w.z + p[2] * cos - p[0] * sin) as i32 as i16;
            c.dim.pos[1] = w.y as i32 as i16;
        }
    }

    /// `func_809BE798`: any of his colliders' attacks landed: Link knocked away (10, 5 up) a
    /// quarter or three eighths of a turn off his course, to Link's side; circling he turns about
    /// (40 frames); spinning towards Link's side, his way round flips and 4 more frames.
    fn check_at_hit(&mut self, play: &mut PlayState) {
        if self.collider_cylinders[0].base.at_flags & AT_HIT != 0 || self.collider_cylinders[1].base.at_flags & AT_HIT != 0 || self.collider.base.at_flags & AT_HIT != 0 {
            self.collider_cylinders[0].base.at_flags &= !AT_HIT;
            self.collider_cylinders[1].base.at_flags &= !AT_HIT;
            self.collider.base.at_flags &= !AT_HIT;
            let yaw_diff = self.actor.yaw_towards_player.wrapping_sub(self.actor.world_rot.y);
            let effect_rot: i16 = if yaw_diff > 0x4000 {
                0x4000
            } else if yaw_diff > 0 {
                0x6000
            } else if yaw_diff < -0x4000 {
                -0x4000
            } else {
                -0x6000
            };
            actor_set_player_knockback_large_no_damage(play, 10.0, self.actor.world_rot.y.wrapping_add(effect_rot), 5.0);
            if self.action == Action::Circle {
                self.setup_turn_around();
                self.unk_196 = 40;
            } else if self.action == Action::Spin && (effect_rot as i32 * self.unk_194 as i32) > 0 {
                self.unk_194 = -self.unk_194;
                self.unk_196 += 4;
            }
        }
    }

    /// `EnBigokuta_UpdateDamage`: his back hit with damage or a reaction: reaction 1 stuns him
    /// (not again while stunned), 0xF throws him up; the rest hurt him only from behind (Link not
    /// within 0x4000 of his face).
    fn update_damage(&mut self, play: &mut PlayState) {
        if self.collider.base.ac_flags & AC_HIT == 0 {
            return;
        }
        self.collider.base.ac_flags &= !AC_HIT;
        let c = &self.actor.col_chk_info;
        if c.damage_reaction == 0 && c.damage == 0 {
            return;
        }
        if c.damage_reaction == 1 {
            if self.action != Action::Stunned {
                self.setup_stunned(play);
            }
        } else if c.damage_reaction == 0xF {
            self.setup_bounce();
        } else if !is_facing_player(&self.actor, 0x4000) {
            if self.actor.apply_damage() == 0 {
                audio_play_actor_sfx2(play, NA_SE_EN_DAIOCTA_DEAD);
                enemy_start_finishing_blow(play, &self.actor);
            } else {
                audio_play_actor_sfx2(play, NA_SE_EN_DAIOCTA_DAMAGE);
            }
            self.setup_hurt();
        }
    }

    fn run_action(&mut self, play: &mut PlayState) {
        match self.action {
            Action::IntroWait => self.intro_wait(play),
            Action::IntroJump => self.intro_jump(play),
            Action::IntroTurn => self.intro_turn(play),
            Action::WaitToFight => self.wait_to_fight(play),
            Action::Circle => self.circle(play),
            Action::Bounce => self.bounce(play),
            Action::TurnAround => self.turn_around(play),
            Action::Stunned => self.stunned(play),
            Action::Hurt => self.hurt(play),
            Action::Die => self.die(play),
            Action::Spin => self.spin(play),
            Action::Sink => self.sink(play),
            Action::Rise => self.rise(),
        }
    }

    /// `EnBigokuta_Draw`'s choice: translucent once he's shrinking (dying, both counts out).
    pub fn drawn_xlu(&self) -> bool {
        self.action == Action::Die && self.unk_196 == 0 && self.unk_198 == 0
    }

    /// `EnBigokuta_OverrideLimbDraw`'s env colours: limb 10's (255, or dying `count × 255/38`) and
    /// limb 15's (stunned or hurt: `sin` of the count's place in a 12- or 6-frame cycle × 125 +
    /// 130), each grey with that alpha (limb 15's 255).
    pub fn limb_envs(&self) -> (u8, Option<u8>) {
        use std::f32::consts::PI;
        let head = match self.action {
            Action::Stunned => {
                let t = self.unk_196 % 12;
                let f = if t >= 8 { (12 - t) as f32 * (PI / 8.0) } else { t as f32 * (PI / 16.0) };
                Some(((f.sin() * 125.0) + 130.0) as i32 as u8)
            }
            Action::Hurt => {
                let t = self.unk_196 % 6;
                let f = if t >= 4 { (6 - t) as f32 * (PI / 4.0) } else { t as f32 * (PI / 8.0) };
                Some(((f.sin() * 125.0) + 130.0) as i32 as u8)
            }
            _ => None,
        };
        let body = if self.action == Action::Die { (self.unk_196 as f32 * (255.0f32 / 38.0)) as i32 as u8 } else { 255 };
        (body, head)
    }

    /// `EnBigokuta_OverrideLimbDraw`'s limb 17 dying: swelling across, then along, then back.
    pub fn swell_scale(&self) -> Option<Vec3> {
        if self.action != Action::Die {
            return None;
        }
        let t = self.unk_198;
        Some(if t < 5 {
            Vec3::new((t as f32 * 0.2 * 0.25) + 1.0, 1.0, 1.0)
        } else if t < 8 {
            let f = (t - 5) as f32 * (1.0 / 12.0);
            Vec3::new(1.25 - f, 1.0 + f, 1.0 + f)
        } else {
            let f = (t - 8) as f32 * 0.125;
            Vec3::new(1.0, 1.25 - f, 1.25 - f)
        })
    }

    /// The translucent draw's env alpha: `scale.y × (255 / 0.033)`.
    pub fn xlu_alpha(&self) -> u8 {
        (self.actor.scale.y * (255.0f32 / 0.033)) as i32 as u8
    }
}

/// `render_state`'s layout: switches XLU, limb 15 set, limb 17 scaled, squashed, then limb 10's
/// env, limb 15's, the XLU alpha; values limb 17's scale, the squash's x and y turns.
mod rs {
    pub const ENV_BODY: usize = 4;
    pub const ENV_HEAD: usize = 5;
    pub const ALPHA: usize = 6;
    pub const SWELL: usize = 0;
    pub const SQUASH: usize = 3;
}

impl ActorImpl for EnBigokuta {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnBigokuta_Update`: see the module's docs.
    fn update(&mut self, play: &mut PlayState) {
        self.check_at_hit(play);
        self.update_damage(play);
        self.run_action(play);
        self.turn_platform(play);
        self.update_colliders();
        // @bug (game): dying, this runs after `func_809BE26C`'s `CAM_SET_DUNGEON0` and
        // `Camera_SetStateFlag`: the request (the same priority, 5) is turned down that frame, but
        // `CAM_STATE_CHECK_BG` ends off.
        play.camera_change_setting(CAM_ID_MAIN, CAM_SET_BIG_OCTO);
        if let Some(c) = play.camera_mut(CAM_ID_MAIN) {
            c.state_flags &= !CAM_STATE_CHECK_BG;
        }
        if self.collider_cylinders[0].base.at_flags & AT_ON != 0 {
            if self.action != Action::Stunned {
                for (i, c) in self.collider_cylinders.iter_mut().enumerate() {
                    play.collision_check_set_at(&self.actor, 1 + i as u8, c);
                }
                self.actor.flags |= ACTOR_FLAG_SFX_FOR_PLAYER_BODY_HIT;
            } else {
                for (i, c) in self.collider_cylinders.iter_mut().enumerate() {
                    play.collision_check_set_oc(&self.actor, 1 + i as u8, c);
                }
            }
            for (i, c) in self.collider_cylinders.iter_mut().enumerate() {
                play.collision_check_set_ac(&self.actor, 1 + i as u8, c);
            }
            if self.collider.base.ac_flags & AC_ON != 0 {
                play.collision_check_set_ac(&self.actor, 0, &mut self.collider);
            }
        }
        if self.collider.base.ac_flags & AC_ON != 0 {
            play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
        } else {
            play.collision_check_set_at(&self.actor, 0, &mut self.collider);
        }
        self.actor.set_focus(self.actor.scale.y * 25.0 * 100.0);
        self.spawn_ripple(play);
    }

    /// `EnBigokuta_Destroy`: the colliders (nothing to free).
    fn destroy(&mut self, _play: &mut PlayState) {}

    /// `EnBigokuta_Draw`'s `Rand_ZeroOne`s: hurt (count 8 and up) or dying (10 and up) on odd
    /// counts, a squash 0.79 by 1.3 along a random axis (turned x, then y).
    fn draw_update(&mut self, play: &mut PlayState) {
        self.squash = None;
        if !self.drawn_xlu() && self.unk_196 & 1 != 0 && ((self.action == Action::Hurt && self.unk_196 >= 8) || (self.action == Action::Die && self.unk_196 >= 10)) {
            let rot_x = play.rand.zero_one() * (std::f32::consts::PI * 2.0);
            let rot_y = play.rand.zero_one() * (std::f32::consts::PI * 2.0);
            self.squash = Some((rot_x, rot_y));
        }
    }

    fn render_state(&self) -> RenderState {
        let mut r = RenderState::of(&self.actor);
        r.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        let (body, head) = self.limb_envs();
        let swell = self.swell_scale();
        r.switches = vec![self.drawn_xlu() as u32, head.is_some() as u32, swell.is_some() as u32, self.squash.is_some() as u32, body as u32, head.unwrap_or(0) as u32, self.xlu_alpha() as u32];
        let s = swell.unwrap_or(Vec3::ONE);
        let (sx, sy) = self.squash.unwrap_or((0.0, 0.0));
        r.values = vec![s.x, s.y, s.z, sx, sy];
        r
    }

    /// `EnBigokuta_Draw`: see the module's docs. The squash (`Matrix_RotateY`, `_RotateX`,
    /// `Matrix_Scale(0.79, 1.3, 0.79)`, and back) on the actor's matrix; limb 17's scale before its
    /// own transform.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let (Some(joints), Some(skeleton)) = (&rs.joints, &self.skeleton) else { return };
        let (sw, v) = (&rs.switches, &rs.values);
        if sw.len() < 7 || v.len() < 5 {
            return;
        }
        let env = |c: [u8; 4]| {
            let mut sv = SegmentValues::default();
            sv.env[SEG_ENV as usize] = Some(c);
            DrawParams { segments: Some(sv), ..Default::default() }
        };
        let model = actor_draw_matrix(rs);
        if sw[0] != 0 {
            let bones = draw_opa_pose(skeleton, &joints.rot, |_, _, _| LimbDraw::Default(Mat4::IDENTITY), |_, _| {});
            out.xlu.push(DrawCmd { mesh: MeshKey::named(keys::bake(BAKE_XLU)), transform: model, bones, params: env([0, 0, 0, sw[rs::ALPHA] as u8]) });
            return;
        }
        let mut m = model;
        if sw[3] != 0 {
            let (rx, ry) = (v[rs::SQUASH], v[rs::SQUASH + 1]);
            m = m * Mat4::from_rotation_y(ry) * Mat4::from_rotation_x(rx) * Mat4::from_scale(Vec3::new(0.78999996, 1.3, 0.78999996)) * Mat4::from_rotation_x(-rx) * Mat4::from_rotation_y(-ry);
        }
        let swell = (sw[2] != 0).then(|| Vec3::new(v[rs::SWELL], v[rs::SWELL + 1], v[rs::SWELL + 2]));
        let bones = draw_opa_pose(
            skeleton,
            &joints.rot,
            |limb, _, _| match (limb, swell) {
                (LIMB_SWELL, Some(s)) => LimbDraw::Default(Mat4::from_scale(s)),
                _ => LimbDraw::Default(Mat4::IDENTITY),
            },
            |_, _| {},
        );
        let body = sw[rs::ENV_BODY] as u8;
        let body_env = [body, body, body, body];
        let back_env = if sw[1] != 0 {
            let h = sw[rs::ENV_HEAD] as u8;
            [h, h, h, 255]
        } else {
            body_env
        };
        for (bake, c) in [(BAKE_FRONT, [255; 4]), (BAKE_BODY, body_env), (BAKE_BACK, back_env)] {
            out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(bake)), transform: m, bones: bones.clone(), params: env(c) });
        }
    }

    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        match id {
            0 => Some(ColliderMut::JntSph(&mut self.collider)),
            1 | 2 => Some(ColliderMut::Cylinder(&mut self.collider_cylinders[id as usize - 1])),
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
