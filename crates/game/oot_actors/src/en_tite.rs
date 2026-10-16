//! `En_Tite` (`ovl_En_Tite/z_en_tite.c`): the Tektites (GAME-09 milestone 2). Four red ones wait at
//! the bottom of Death Mountain Trail (`spot16`, params -1); blue ones (-2) walk on water in
//! Zora's River, Lake Hylia, a grotto and the Ice Cavern; `En_Encount1` spawns red ones.
//!
//! - **Idle** (`EnTite_Idle`): 15 to 44 frames, then once Link is within 300 across and at most 80
//!   above, it turns to him (`EnTite_TurnTowardPlayer`: turning faster the further he's off, its
//!   steps sounding every 8 frames). Facing him within 3640: within 180 (and 80 up) it **attacks**
//!   (`EnTite_Attack`), further it **hops at him** (`EnTite_MoveTowardPlayer`: 10 up, 4 on, one to
//!   three more hops, no hurting).
//! - **The attack**: a crouch (`object_tite_Anim_00083C`), then a lunge (8 up, 4 on) with its
//!   sphere hurting (8 damage); a hit on Link (`NA_SE_PL_BODY_HIT` on him unless it bounced) or
//!   anything else sends it back at -6 (`EnTite_Recoil`); landed, one to three more lunges, then it
//!   turns again. @bug (game): out of its culling volume (`ACTOR_FLAG_INSIDE_CULLING_VOLUME`) a
//!   lunge "recoils" at once, as if it hit.
//! - **Hit** (`EnTite_CheckDamage`, from idle on): nuts, the boomerang and the hookshot stun it
//!   (`EnTite_Stunned`: blue 80 frames, knocked back 6), ice too (and the ice chunks over it,
//!   `EffectSsEnIce`); fire and light magic do nothing (and hold it still that frame); other
//!   damage flashes it red (8 frames; no damage while it's still red), `NA_SE_EN_TEKU_DAMAGE`,
//!   knocked back (`EnTite_Recoil`), or flips it back up if it's on its back. Recoiling
//!   (`TEKTITE_RECOIL`, under `TEKTITE_IDLE`) it isn't hit. Red: 2 health (the Kokiri Sword's 1
//!   twice); blue: 4.
//! - **The hammer**: the hammer's floor hit (`actorCtx.unk_02`) within 400 of a grounded Tektite
//!   flips it onto its back (`EnTite_FlipOnBack`, 500 frames twitching), or back up
//!   (`EnTite_FlipUpright`).
//! - **Death** (`EnTite_DeathCry`): its cry (`EffectSsDeadSound`, `NA_SE_EN_TEKU_DEAD`, 40 frames),
//!   its 24 limbs (`BodyBreak`) thrown off as `En_Part` of kind `params + 0xB` (10 red, 9 blue)
//!   once it's drawn, and a drop from table 4 (red) or 14 (blue) (`EnTite_FallApart`).
//! - **The blue one on water**: it floats on the surface (no gravity, `depthInWater` back to 0),
//!   lunges and hops from it (`NA_SE_EN_TEKU_JUMP_WATER`), lands on it (`NA_SE_EN_TEKU_LAND_WATER`,
//!   `_WATER2`), a fast landing dipping under and floating back up (`TEKTITE_SUBMERGED`); its feet
//!   ripple the water (every 8 frames each, or every frame falling).
//!
//! The whole overlay is ported. Its draw (`EnTite_Draw`) is `object_tite_Skel_003A20` with the red
//! or the blue textures on segments 8 to 10 (two bakes), the circle shadow the usual stand-in;
//! `draw_update` (`EnTite_PostLimbDraw`) puts its sphere, its feet (2800 along limbs 8, 13, 18 and
//! 23) and the body break's pieces, and spawns the ice chunks. Not ported: the water ripples
//! (`EffectSsGRipple`, logged where the C spawns one). Nothing sets the hammer's `unk_02` yet (the
//! hammer isn't ported).
//!
//! Params: `params` whole, the type (`EnTiteType`: -2 `TEKTITE_BLUE`, walks on water, 4 health;
//! -1 `TEKTITE_RED`, 2 health; another value is a red one whose parts are `En_Part` kind
//! `params + 0xB`).
//! Flags: none.

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::bgcheck::BGCHECK_Y_MIN;
use eng_collision::math3d::Sphere16;
use eng_gfx::{DrawCmd, MeshKey};
use eng_math::{smooth_step_to_f, smooth_step_to_s, vec3f_yaw};
use glam::{Mat4, Vec3};
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_ENEMY, ActorImpl, ActorProfile, actor_spawn_floor_dust_ring, audio_play_actor_sfx2, audio_play_actor_sfx_at, func_80033480, func_800355b8, func_800359b8};
use oot_game::actor_schema::{ActorSchema, Field, Kind};
use oot_game::audio::sfx::{NA_SE_EN_DODO_M_GND, NA_SE_EN_GOMA_JR_FREEZE, NA_SE_PL_BODY_HIT};
use oot_game::collision_check::*;
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::skelanime_std::{ANIMMODE_LOOP, Anim, LimbDraw, SkelAnimeStd, draw_opa_pose};

use crate::en_part::{BODYBREAK_OBJECT_SLOT_DEFAULT, BODYBREAK_STATUS_FINISHED, BodyBreak, DList};

/// `ACTOR_EN_TITE` (`actor_table.h`: 0x001B).
pub const ACTOR_EN_TITE: i16 = 0x001B;
/// `OBJECT_TITE` (`object_table.h`: 0x0016).
pub const OBJECT: &str = "object_tite";
const SKEL: &str = "object_tite_Skel_003A20";

/// `En_Tite_Profile`: `ACTORCAT_ENEMY`, attention-enabled, hostile, always updating.
pub const PROFILE: ActorProfile =
    ActorProfile { id: ACTOR_EN_TITE, name: "En_Tite", category: ACTORCAT_ENEMY, flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE | ACTOR_FLAG_UPDATE_CULLING_DISABLED, object: OBJECT, schema: &SCHEMA };

/// `this->actor.params`: the type (`EnTiteType`).
pub const TYPE: Field = Field::whole("params", Kind::Named(&[(-2, "TEKTITE_BLUE"), (-1, "TEKTITE_RED")]), "blue (walks on water, 4 health) or red (2 health)");

/// En_Tite's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema { fields: &[TYPE], flags: &[], todo: false };

/// `EnTiteType` (`z_en_tite.h`).
pub const TEKTITE_BLUE: i16 = -2;
pub const TEKTITE_RED: i16 = -1;

/// `EnTiteAction`.
pub const TEKTITE_DEATH_CRY: u8 = 0x0;
pub const TEKTITE_UNK_1: u8 = 0x1;
pub const TEKTITE_UNK_2: u8 = 0x2;
pub const TEKTITE_RECOIL: u8 = 0x3;
pub const TEKTITE_UNK_4: u8 = 0x4;
pub const TEKTITE_FALL_APART: u8 = 0x5;
pub const TEKTITE_IDLE: u8 = 0x6;
pub const TEKTITE_STUNNED: u8 = 0x7;
pub const TEKTITE_UNK_8: u8 = 0x8;
pub const TEKTITE_ATTACK: u8 = 0x9;
pub const TEKTITE_TURN_TOWARD_PLAYER: u8 = 0xA;
pub const TEKTITE_UNK9: u8 = 0xB;
pub const TEKTITE_MOVE_TOWARD_PLAYER: u8 = 0xC;

/// `EnTiteAttackState` (`vAttackState`). (The enum's comment gives `TEKTITE_SUBMERGED` 0x2 too;
/// its value is 3.)
pub const TEKTITE_BEGIN_LUNGE: i16 = 0x0;
pub const TEKTITE_MID_LUNGE: i16 = 0x1;
pub const TEKTITE_LANDED: i16 = 0x2;
pub const TEKTITE_SUBMERGED: i16 = 0x3;

/// `EnTiteFlipState`.
pub const TEKTITE_INITIAL: u8 = 0x0;
pub const TEKTITE_UNFLIPPED: u8 = 0x1;
pub const TEKTITE_FLIPPED: u8 = 0x2;

// The sounds (`include/tables/sfx/enemybank_table.h`).
/// `NA_SE_EN_TEKU_WALK_WATER` (0x3835).
pub const NA_SE_EN_TEKU_WALK_WATER: u16 = 0x3835;
/// `NA_SE_EN_STAL_JUMP` (0x386C).
pub const NA_SE_EN_STAL_JUMP: u16 = 0x386C;
/// `NA_SE_EN_TEKU_DAMAGE` (0x386D).
pub const NA_SE_EN_TEKU_DAMAGE: u16 = 0x386D;
/// `NA_SE_EN_TEKU_DEAD` (0x386E).
pub const NA_SE_EN_TEKU_DEAD: u16 = 0x386E;
/// `NA_SE_EN_TEKU_WALK` (0x386F).
pub const NA_SE_EN_TEKU_WALK: u16 = 0x386F;
/// `NA_SE_EN_TEKU_REVERSE` (0x3888).
pub const NA_SE_EN_TEKU_REVERSE: u16 = 0x3888;
/// `NA_SE_EN_TEKU_LAND_WATER` (0x388A).
pub const NA_SE_EN_TEKU_LAND_WATER: u16 = 0x388A;
/// `NA_SE_EN_TEKU_JUMP_WATER` (0x388E).
pub const NA_SE_EN_TEKU_JUMP_WATER: u16 = 0x388E;
/// `NA_SE_EN_TEKU_LAND_WATER2` (0x388F).
pub const NA_SE_EN_TEKU_LAND_WATER2: u16 = 0x388F;

/// `NAVI_ENEMY_RED_TEKTITE`, `NAVI_ENEMY_BLUE_TEKTITE` (`actor.h`: 0x45, 0x46).
pub const NAVI_ENEMY_RED_TEKTITE: u8 = 0x45;
pub const NAVI_ENEMY_BLUE_TEKTITE: u8 = 0x46;

/// `ATTENTION_RANGE_3` (`actor.h`).
const ATTENTION_RANGE_3: u8 = 3;
/// `UPDBGCHECKINFO_FLAG_6` (`actor.h`: 1 << 6, no water ripples from `Actor_UpdateBgCheckInfo`).
const UPDBGCHECKINFO_FLAG_6: u32 = 1 << 6;
/// `DEADSOUND_REPEAT_MODE_OFF` (`z_eff_ss_dead_sound.h`: 1).
const DEADSOUND_REPEAT_MODE_OFF: i16 = 1;

/// `COLLECTIBLE_DROP_RANDOM_PARAMS(COLLECTIBLE_DROP_TABLE_4 / _14, false)` (`z_en_item00.h`:
/// the table × 16).
const DROP_TABLE_4: i16 = 4 * 16;
const DROP_TABLE_14: i16 = 14 * 16;

/// `sJntSphElementsInit`: its body (limb 0: the actor's matrix, 1500 up, 20), hurting 8, hit by
/// Player's 0xFFCFFFFF, hookable.
const JNT_SPH_ELEMENTS: [ColliderJntSphElementInit; 1] = [ColliderJntSphElementInit {
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0xFFCF_FFFF, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x08 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: HIT_BACKLASH_NONE, defense: 0x00 },
        at_elem_flags: ATELEM_ON | ATELEM_SFX_NORMAL,
        ac_elem_flags: ACELEM_ON | ACELEM_HOOKABLE,
        oc_elem_flags: OCELEM_ON,
    },
    limb: 0,
    model_sphere: Sphere16 { center: [0, 1500, 0], radius: 20 },
    scale: 100,
}];

/// `sJntSphInit`.
const JNT_SPH_INIT: ColliderInit =
    ColliderInit { col_type: COL_MATERIAL_HIT6, at_flags: AT_ON | AT_TYPE_ENEMY, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_JNTSPH };

/// `sDamageTable`.
pub static S_DAMAGE_TABLE: DamageTable = DamageTable {
    table: [
        dmg_entry(0, 0x1), // Deku nut
        dmg_entry(2, 0x0), // Deku stick
        dmg_entry(1, 0x0), // Slingshot
        dmg_entry(2, 0x0), // Explosive
        dmg_entry(0, 0x1), // Boomerang
        dmg_entry(2, 0x0), // Normal arrow
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
        dmg_entry(0, 0xE), // Fire magic
        dmg_entry(3, 0xF), // Ice magic
        dmg_entry(0, 0xE), // Light magic
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

/// `sFootOffset`: a foot's place on its limb.
const FOOT_OFFSET: Vec3 = Vec3::new(2800.0, -200.0, 0.0);

/// `sIceChunks`: where the ice chunks go over a frozen Tektite.
const ICE_CHUNKS: [Vec3; 12] = [
    Vec3::new(20.0, 20.0, 0.0),
    Vec3::new(10.0, 40.0, 10.0),
    Vec3::new(-10.0, 40.0, 10.0),
    Vec3::new(-20.0, 20.0, 0.0),
    Vec3::new(10.0, 40.0, -10.0),
    Vec3::new(-10.0, 40.0, -10.0),
    Vec3::new(0.0, 20.0, -20.0),
    Vec3::new(10.0, 0.0, 10.0),
    Vec3::new(10.0, 0.0, -10.0),
    Vec3::new(0.0, 20.0, 20.0),
    Vec3::new(-10.0, 0.0, 10.0),
    Vec3::new(-10.0, 0.0, -10.0),
];

/// The limbs `EnTite_PostLimbDraw` takes the feet from (1-based).
const LIMB_BACK_RIGHT_FOOT: usize = 8;
const LIMB_FRONT_RIGHT_FOOT: usize = 13;
const LIMB_BACK_LEFT_FOOT: usize = 18;
const LIMB_FRONT_LEFT_FOOT: usize = 23;

/// `object_tite`'s limb lists (`object_tite.xml`): the skeleton's table names them by address.
pub const DLISTS: [&str; 9] = [
    "object_tite_DL_002FF0",
    "object_tite_DL_003160",
    "object_tite_DL_003290",
    "object_tite_DL_003330",
    "object_tite_DL_003460",
    "object_tite_DL_003500",
    "object_tite_DL_003630",
    "object_tite_DL_0036D0",
    "object_tite_DL_003800",
];

/// The textures `EnTite_Draw` puts on segments 8, 9 and 10: the blue's, the red's.
pub const TEXTURES_BLUE: [&str; 3] = ["object_tite_Tex_001300", "object_tite_Tex_001700", "object_tite_Tex_001900"];
pub const TEXTURES_RED: [&str; 3] = ["object_tite_Tex_001B00", "object_tite_Tex_001F00", "object_tite_Tex_002100"];
/// Their segments.
pub const SEGMENTS: [u8; 3] = [0x08, 0x09, 0x0A];

const BAKE_RED: &str = "En_Tite/red";
const BAKE_BLUE: &str = "En_Tite/blue";

/// The segments 8 to 10 with the blue's or the red's textures.
pub fn texture_segments(blue: bool) -> Vec<(u8, BakeSegment)> {
    let tex = if blue { TEXTURES_BLUE } else { TEXTURES_RED };
    SEGMENTS.iter().zip(tex).map(|(&seg, t)| (seg, BakeSegment::Texture { file: OBJECT.into(), symbol: t.into() })).collect()
}

/// `EnTite_Draw`'s meshes: the skeleton after `Gfx_SetupDL_25Opa`, red and blue.
pub fn bakes() -> Vec<MeshBake> {
    [(BAKE_RED, false), (BAKE_BLUE, true)]
        .into_iter()
        .map(|(name, blue)| MeshBake {
            name: name.into(),
            object: OBJECT.into(),
            segments: texture_segments(blue),
            prelude: Vec::new(),
            body: BakeBody::Skeleton { file: OBJECT.into(), symbol: SKEL.into(), limbs: Vec::new() },
        })
        .collect()
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Func {
    Idle,
    Attack,
    TurnTowardPlayer,
    MoveTowardPlayer,
    Recoil,
    Stunned,
    DeathCry,
    FallApart,
    FlipOnBack,
    FlipUpright,
}

impl Func {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Func::Idle => "EnTite_Idle",
            Func::Attack => "EnTite_Attack",
            Func::TurnTowardPlayer => "EnTite_TurnTowardPlayer",
            Func::MoveTowardPlayer => "EnTite_MoveTowardPlayer",
            Func::Recoil => "EnTite_Recoil",
            Func::Stunned => "EnTite_Stunned",
            Func::DeathCry => "EnTite_DeathCry",
            Func::FallApart => "EnTite_FallApart",
            Func::FlipOnBack => "EnTite_FlipOnBack",
            Func::FlipUpright => "EnTite_FlipUpright",
        }
    }
}

struct Anims {
    /// `object_tite_Anim_0012E4`: idle (14 frames).
    idle: Anim,
    /// `object_tite_Anim_00083C`: the lunge's crouch (6 frames).
    lunge: Anim,
    /// `object_tite_Anim_000A14`: turning (8 frames).
    turn: Anim,
    /// `object_tite_Anim_000C70`: the hop (11 frames).
    jump: Anim,
    /// `object_tite_Anim_0004F8`, `object_tite_Anim_00069C`: `D_80B1B634[1]`, `[2]` (mid-lunge,
    /// landed).
    anim_0004f8: Anim,
    anim_00069c: Anim,
}

impl Anims {
    fn load(play: &PlayState) -> Option<Anims> {
        let a = play.assets.clone()?;
        let get = |s: &str| a.animation(OBJECT, s).map_err(|e| log::error!("En_Tite: {e:#}")).ok();
        Some(Anims {
            idle: get("object_tite_Anim_0012E4")?,
            lunge: get("object_tite_Anim_00083C")?,
            turn: get("object_tite_Anim_000A14")?,
            jump: get("object_tite_Anim_000C70")?,
            anim_0004f8: get("object_tite_Anim_0004F8")?,
            anim_00069c: get("object_tite_Anim_00069C")?,
        })
    }
}

/// `ABS` of an s16 in an expression (an `int`: -0x8000 is 0x8000).
fn abs_i(v: i16) -> i32 {
    (v as i32).abs()
}

pub struct EnTite {
    pub actor: Actor,
    pub skel: SkelAnimeStd,
    skeleton: Option<Arc<Skeleton>>,
    anims: Option<Anims>,
    /// `action` (`EnTiteAction`).
    pub action: u8,
    /// `flipState` (`EnTiteFlipState`).
    pub flip_state: u8,
    pub func: Func,
    pub body_break: BodyBreak,
    /// `unk_2DC`: its `Actor_UpdateBgCheckInfo` flags.
    pub unk_2dc: u32,
    /// `actionVar1`: `vIdleTimer` (Idle), `vAttackState` (Attack), `vOnBackTimer` (FlipOnBack).
    pub action_var1: i16,
    /// `actionVar2` (a `u8`): `vQueuedJumps` (Attack, MoveTowardPlayer), `vLegTwitchTimer`
    /// (FlipOnBack).
    pub action_var2: u8,
    pub spawn_ice_timer: u8,
    pub damage_reaction: u8,
    pub collider: ColliderJntSph,
    pub front_left_foot_pos: Vec3,
    pub front_right_foot_pos: Vec3,
    pub back_right_foot_pos: Vec3,
    pub back_left_foot_pos: Vec3,
    /// `shape.shadowScale`.
    pub shadow_scale: f32,
}

impl EnTite {
    fn anim(&self, f: impl Fn(&Anims) -> &Anim) -> Option<Anim> {
        self.anims.as_ref().map(|a| f(a).clone())
    }

    /// `Animation_Change(anim, speed, 0, last, mode, morph)`.
    fn change(&mut self, f: impl Fn(&Anims) -> &Anim, speed: f32, mode: u8, morph: f32) {
        if let Some(a) = self.anim(f) {
            let last = a.last_frame();
            self.skel.change(a, speed, 0.0, last, mode, morph);
        }
    }

    /// `Animation_PlayOnce`.
    fn play_once(&mut self, f: impl Fn(&Anims) -> &Anim) {
        if let Some(a) = self.anim(f) {
            self.skel.play_once(a);
        }
    }

    fn blue(&self) -> bool {
        TYPE.get(self.actor.params) == TEKTITE_BLUE
    }

    /// The blue one in water (`params == TEKTITE_BLUE && bgCheckFlags & BGCHECKFLAG_WATER`).
    fn blue_in_water(&self) -> bool {
        self.blue() && self.actor.bg_check_flags & BGCHECKFLAG_WATER != 0
    }

    /// On the ground (`BGCHECKFLAG_GROUND | BGCHECKFLAG_GROUND_TOUCH`), or the blue one in water.
    fn grounded_or_blue_in_water(&self) -> bool {
        self.actor.bg_check_flags & (BGCHECKFLAG_GROUND | BGCHECKFLAG_GROUND_TOUCH) != 0 || self.blue_in_water()
    }

    /// Link far (`xzDistToPlayer > 300 && yDistToPlayer > 80`). @bug (game): `&&` where Idle's
    /// check (`< 300 && <= 80`) has `||` for its opposite: a Tektite on Link's level keeps after
    /// him however far he goes; only Link far and over 80 above it sends it back to idle.
    fn player_far(&self) -> bool {
        self.actor.xz_dist_to_player > 300.0 && self.actor.y_dist_to_player > 80.0
    }

    /// `EnTite_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // sInitChain: naviEnemyId NAVI_ENEMY_RED_TEKTITE, lockOnArrowOffset 2000, minVelocityY -40,
        // gravity -1.
        actor.navi_enemy_id = NAVI_ENEMY_RED_TEKTITE;
        actor.target_arrow_offset = 2000.0;
        actor.min_velocity_y = -40.0;
        actor.gravity = -1.0;
        actor.target_mode = ATTENTION_RANGE_3;
        actor.scale = Vec3::splat(0.01);
        let skeleton = play.assets.clone().and_then(|a| a.skeleton(OBJECT, SKEL).map_err(|e| log::error!("En_Tite: {e:#}")).ok());
        let anims = Anims::load(play);
        // SkelAnime_Init(.., object_tite_Skel_003A20, object_tite_Anim_0012E4, .., 25): 24 limbs.
        let skel = SkelAnimeStd::init_flex(24, anims.as_ref().map(|a| a.idle.clone()));
        // ActorShape_Init(&shape, -200, ActorShadow_DrawCircle, 70).
        actor.shape_y_offset = -200.0;
        actor.col_chk_info.damage_table = Some(&S_DAMAGE_TABLE);
        actor.focus_pos = actor.world_pos;
        actor.focus_pos.y += 20.0;
        actor.col_chk_info.health = 2;
        actor.col_chk_info.mass = MASS_HEAVY;
        let collider = ColliderJntSph::new(&JNT_SPH_INIT, &JNT_SPH_ELEMENTS);
        let mut this = EnTite {
            actor,
            skel,
            skeleton,
            anims,
            action: TEKTITE_IDLE,
            flip_state: TEKTITE_INITIAL,
            func: Func::Idle,
            body_break: BodyBreak { val: BODYBREAK_STATUS_FINISHED, ..Default::default() },
            unk_2dc: UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2 | UPDBGCHECKINFO_FLAG_3 | UPDBGCHECKINFO_FLAG_4,
            action_var1: 0,
            action_var2: 0,
            spawn_ice_timer: 0,
            damage_reaction: 0,
            collider,
            front_left_foot_pos: Vec3::ZERO,
            front_right_foot_pos: Vec3::ZERO,
            back_right_foot_pos: Vec3::ZERO,
            back_left_foot_pos: Vec3::ZERO,
            shadow_scale: 70.0,
        };
        if this.blue() {
            // Don't use the actor engine's ripple spawning code.
            this.unk_2dc |= UPDBGCHECKINFO_FLAG_6;
            this.actor.col_chk_info.health = 4;
            this.actor.navi_enemy_id += NAVI_ENEMY_BLUE_TEKTITE - NAVI_ENEMY_RED_TEKTITE;
        }
        this.setup_idle(play);
        Box::new(this)
    }

    /// `EffectSsGRipple_Spawn`: not ported.
    fn effect_ss_g_ripple_spawn(pos: Vec3, radius: i16, radius_max: i16, life: i16) {
        log::debug!("En_Tite: EffectSsGRipple isn't ported (at {pos}, {radius} to {radius_max}, after {life})");
    }

    /// `func_80033480` at the four feet: their dust.
    fn feet_dust(&self, play: &mut PlayState) {
        for pos in [self.front_left_foot_pos, self.front_right_foot_pos, self.back_right_foot_pos, self.back_left_foot_pos] {
            func_80033480(play, pos, 1.0, 2, 80, 15, 1);
        }
    }

    /// `func_800355B8` at the four feet: their sparkles.
    fn feet_sparkles(&self, play: &mut PlayState) {
        for pos in [self.front_left_foot_pos, self.front_right_foot_pos, self.back_right_foot_pos, self.back_left_foot_pos] {
            func_800355b8(play, pos);
        }
    }

    /// `EnTite_SetupIdle`.
    fn setup_idle(&mut self, play: &mut PlayState) {
        // Animation_MorphToLoop(object_tite_Anim_0012E4, 4): a 4-frame morph into its first frame.
        self.change(|a| &a.idle, 1.0, ANIMMODE_LOOP, 4.0);
        self.action = TEKTITE_IDLE;
        // vIdleTimer.
        self.action_var1 = play.rand.s16_offset(15, 30);
        self.actor.speed_xz = 0.0;
        self.func = Func::Idle;
    }

    /// `EnTite_Idle`.
    fn idle(&mut self, _play: &mut PlayState) {
        self.skel.update();
        smooth_step_to_f(&mut self.actor.speed_xz, 0.0, 1.0, 0.5, 0.0);
        if self.blue() {
            if self.actor.bg_check_flags & BGCHECKFLAG_WATER != 0 {
                // Float on water surface.
                self.actor.gravity = 0.0;
                smooth_step_to_f(&mut self.actor.velocity.y, 0.0, 1.0, 2.0, 0.0);
                let target = self.actor.world_pos.y + self.actor.y_dist_to_water;
                smooth_step_to_f(&mut self.actor.world_pos.y, target, 1.0, 2.0, 0.0);
            } else {
                self.actor.gravity = -1.0;
            }
        }
        if self.actor.bg_check_flags & (BGCHECKFLAG_GROUND | BGCHECKFLAG_GROUND_TOUCH) != 0 && self.actor.velocity.y <= 0.0 {
            self.actor.velocity.y = 0.0;
        }
        // vIdleTimer.
        if self.action_var1 > 0 {
            self.action_var1 -= 1;
        } else if self.actor.xz_dist_to_player < 300.0 && self.actor.y_dist_to_player <= 80.0 {
            self.setup_turn_toward_player();
        }
    }

    /// `EnTite_SetupAttack`.
    fn setup_attack(&mut self, play: &mut PlayState) {
        self.play_once(|a| &a.lunge);
        self.action = TEKTITE_ATTACK;
        // vAttackState, vQueuedJumps (a u8).
        self.action_var1 = TEKTITE_BEGIN_LUNGE;
        self.action_var2 = play.rand.s16_offset(1, 3) as u8;
        self.actor.speed_xz = 0.0;
        self.actor.velocity.y = 0.0;
        self.actor.world_rot.y = self.actor.shape_rot.y;
        self.func = Func::Attack;
    }

    /// `EnTite_Attack`.
    fn attack(&mut self, play: &mut PlayState) {
        if self.skel.update() {
            // For deciding whether to change animation.
            let mut attack_state = self.action_var1;
            match self.action_var1 {
                TEKTITE_BEGIN_LUNGE => {
                    // Snap to ground or water, then lunge into the air with some initial speed.
                    self.action_var1 = TEKTITE_MID_LUNGE;
                    if !self.blue_in_water() {
                        if self.actor.floor_height > BGCHECK_Y_MIN {
                            self.actor.world_pos.y = self.actor.floor_height;
                        }
                        audio_play_actor_sfx2(play, NA_SE_EN_STAL_JUMP);
                    } else {
                        self.actor.world_pos.y += self.actor.y_dist_to_water;
                        audio_play_actor_sfx2(play, NA_SE_EN_TEKU_JUMP_WATER);
                    }
                    self.actor.velocity.y = 8.0;
                    self.actor.gravity = -1.0;
                    self.actor.speed_xz = 4.0;
                }
                TEKTITE_MID_LUNGE => {
                    // Continue until it falls onto the ground or the water's surface; snap to it,
                    // or falling fast dip into the water and slow.
                    self.actor.flags |= ACTOR_FLAG_SFX_FOR_PLAYER_BODY_HIT;
                    if self.grounded_or_blue_in_water() && self.actor.velocity.y <= 0.0 {
                        self.action_var1 = TEKTITE_LANDED;
                        if !self.blue_in_water() {
                            if BGCHECK_Y_MIN < self.actor.floor_height {
                                self.actor.world_pos.y = self.actor.floor_height;
                            }
                            self.actor.velocity.y = 0.0;
                            self.actor.speed_xz = 0.0;
                        } else {
                            self.actor.gravity = 0.0;
                            if self.actor.velocity.y < -8.0 {
                                let mut ripple_pos = self.actor.world_pos;
                                ripple_pos.y += self.actor.y_dist_to_water;
                                // TEKTITE_SUBMERGED.
                                self.action_var1 += 1;
                                self.actor.velocity.y *= 0.75;
                                attack_state = self.action_var1;
                                Self::effect_ss_g_ripple_spawn(ripple_pos, 0, 500, 0);
                            } else {
                                self.actor.velocity.y = 0.0;
                                self.actor.speed_xz = 0.0;
                            }
                        }
                        self.actor.world_rot.y = self.actor.shape_rot.y;
                    }
                }
                TEKTITE_LANDED => {
                    // Another lunge if more are queued, else turn.
                    if self.action_var2 != 0 {
                        self.action_var2 -= 1;
                        self.action_var1 = TEKTITE_BEGIN_LUNGE;
                        self.collider.base.at_flags &= !AT_HIT;
                    } else {
                        self.setup_turn_toward_player();
                    }
                }
                TEKTITE_SUBMERGED => {
                    // Floated to the surface?
                    if self.actor.y_dist_to_water == 0.0 {
                        self.action_var1 = TEKTITE_LANDED;
                        attack_state = self.action_var1;
                    }
                }
                _ => {}
            }
            // A new attack state, a new animation (not between submerged and landed).
            if attack_state != self.action_var1 {
                // D_80B1B634[vAttackState] (the 4th on is NULL, never reached).
                match self.action_var1 {
                    0 => self.play_once(|a| &a.lunge),
                    1 => self.play_once(|a| &a.anim_0004f8),
                    2 => self.play_once(|a| &a.anim_00069c),
                    v => log::debug!("En_Tite: D_80B1B634[{v}] is NULL"),
                }
            }
        }

        match self.action_var1 {
            TEKTITE_BEGIN_LUNGE => {
                // Turn a little to Link; turn or idle if he's off or far.
                smooth_step_to_s(&mut self.actor.world_rot.y, self.actor.yaw_towards_player, 1, 1000, 0);
                self.actor.shape_rot.y = self.actor.world_rot.y;
                let angle_to_player = self.actor.yaw_towards_player.wrapping_sub(self.actor.shape_rot.y);
                if self.player_far() {
                    self.setup_idle(play);
                } else if abs_i(angle_to_player) >= 9000 {
                    self.setup_turn_toward_player();
                }
            }
            TEKTITE_MID_LUNGE => {
                // Sparkles at the feet leaving the ground; the attack's sphere until it hits.
                if self.actor.velocity.y >= 5.0 && self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
                    self.feet_sparkles(play);
                }
                if self.collider.base.at_flags & AT_HIT == 0 && self.actor.flags & ACTOR_FLAG_INSIDE_CULLING_VOLUME != 0 {
                    play.collision_check_set_at(&self.actor, 0, &mut self.collider);
                } else {
                    self.collider.base.at_flags &= !AT_HIT;
                    // Animation_MorphToLoop(object_tite_Anim_0012E4, 4).
                    self.change(|a| &a.idle, 1.0, ANIMMODE_LOOP, 4.0);
                    self.actor.speed_xz = -6.0;
                    self.actor.world_rot.y = self.actor.yaw_towards_player;
                    if play.player.is_some() && self.collider.base.at == play.player && self.collider.base.at_flags & AT_BOUNCED == 0 {
                        audio_play_actor_sfx_at(play, play.player.unwrap(), NA_SE_PL_BODY_HIT);
                    }
                    // EnTite_SetupAction(Recoil) alone: `action` stays TEKTITE_ATTACK.
                    self.func = Func::Recoil;
                }
            }
            TEKTITE_LANDED => {
                // Turn a little to Link.
                smooth_step_to_s(&mut self.actor.world_rot.y, self.actor.yaw_towards_player, 1, 1500, 0);
            }
            TEKTITE_SUBMERGED => {
                // Float up to the surface.
                smooth_step_to_f(&mut self.actor.velocity.y, 0.0, 1.0, 2.0, 0.0);
                smooth_step_to_f(&mut self.actor.speed_xz, 0.0, 1.0, 0.5, 0.0);
                let target = self.actor.world_pos.y + self.actor.y_dist_to_water;
                smooth_step_to_f(&mut self.actor.world_pos.y, target, 1.0, 2.0, 0.0);
            }
            _ => {}
        }
        // Dust where its feet land (not in water).
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND_TOUCH != 0 && self.actor.bg_check_flags & BGCHECKFLAG_WATER == 0 {
            self.feet_dust(play);
        }
        // Landed: its speed stopped and the landing's sound.
        if self.blue() {
            if self.actor.bg_check_flags & BGCHECKFLAG_WATER_TOUCH != 0 {
                self.actor.speed_xz = 0.0;
                // (vAttackState: actionVar1, the idle timer if it just went idle.)
                if self.action_var1 == TEKTITE_SUBMERGED {
                    audio_play_actor_sfx2(play, NA_SE_EN_TEKU_LAND_WATER);
                } else {
                    audio_play_actor_sfx2(play, NA_SE_EN_TEKU_LAND_WATER2);
                }
                self.actor.bg_check_flags &= !BGCHECKFLAG_WATER_TOUCH;
            } else if self.actor.bg_check_flags & BGCHECKFLAG_GROUND_TOUCH != 0 {
                audio_play_actor_sfx2(play, NA_SE_EN_DODO_M_GND);
            }
        } else if self.actor.bg_check_flags & BGCHECKFLAG_GROUND_TOUCH != 0 {
            self.actor.speed_xz = 0.0;
            audio_play_actor_sfx2(play, NA_SE_EN_DODO_M_GND);
        }
    }

    /// `EnTite_SetupTurnTowardPlayer`.
    fn setup_turn_toward_player(&mut self) {
        // Animation_PlayLoop(object_tite_Anim_000A14).
        self.change(|a| &a.turn, 1.0, ANIMMODE_LOOP, 0.0);
        self.action = TEKTITE_TURN_TOWARD_PLAYER;
        if self.grounded_or_blue_in_water() && self.actor.velocity.y <= 0.0 {
            self.actor.gravity = 0.0;
            self.actor.velocity.y = 0.0;
            self.actor.speed_xz = 0.0;
        }
        self.func = Func::TurnTowardPlayer;
    }

    /// `EnTite_TurnTowardPlayer`.
    fn turn_toward_player(&mut self, play: &mut PlayState) {
        if self.grounded_or_blue_in_water() && self.actor.velocity.y <= 0.0 {
            self.actor.gravity = 0.0;
            self.actor.velocity.y = 0.0;
            self.actor.speed_xz = 0.0;
        }
        // The turn's speed and the animation's by how far Link is off.
        if self.blue_in_water() {
            self.actor.world_pos.y += self.actor.y_dist_to_water;
        }
        // Actor_WorldYawTowardActor(&this->actor, &GET_PLAYER(play)->actor).
        let player_pos = play.player.and_then(|h| play.actors.actor(h)).map(|p| p.world_pos).unwrap_or(self.actor.world_pos);
        let angle_to_player = vec3f_yaw(self.actor.world_pos, player_pos).wrapping_sub(self.actor.world_rot.y);
        // turnVelocity (an s16): (angleToPlayer / 42) ± 10.
        let turn_velocity = if angle_to_player > 0 { ((angle_to_player as f32 / 42.0) + 10.0) as i32 as i16 } else { ((angle_to_player as f32 / 42.0) - 10.0) as i32 as i16 };
        self.actor.world_rot.y = (self.actor.world_rot.y as i32 + turn_velocity as i32 * 2) as i16;
        self.skel.play_speed = turn_velocity as f32 * 0.01;

        // Its steps' sound once each 8 frames of the animation.
        self.skel.update();
        if (self.skel.cur_frame as i32 as i16) & 7 == 0 {
            if self.blue_in_water() {
                audio_play_actor_sfx2(play, NA_SE_EN_TEKU_WALK_WATER);
            } else {
                audio_play_actor_sfx2(play, NA_SE_EN_TEKU_WALK);
            }
        }

        // Idle with Link far; facing him, attack or hop at him.
        self.actor.shape_rot.y = self.actor.world_rot.y;
        if self.player_far() {
            self.setup_idle(play);
        } else if abs_i(self.actor.yaw_towards_player.wrapping_sub(self.actor.shape_rot.y)) < 3640 {
            // Actor_IsFacingPlayer(&this->actor, 3640).
            if self.actor.xz_dist_to_player <= 180.0 && self.actor.y_dist_to_player <= 80.0 {
                self.setup_attack(play);
            } else {
                self.setup_move_toward_player(play);
            }
        }
    }

    /// The hop's sound (`NA_SE_EN_TEKU_JUMP_WATER` from the water, `NA_SE_EN_STAL_JUMP`).
    fn jump_sfx(&self, play: &mut PlayState) {
        if self.blue_in_water() {
            audio_play_actor_sfx2(play, NA_SE_EN_TEKU_JUMP_WATER);
        } else {
            audio_play_actor_sfx2(play, NA_SE_EN_STAL_JUMP);
        }
    }

    /// `EnTite_SetupMoveTowardPlayer`.
    fn setup_move_toward_player(&mut self, play: &mut PlayState) {
        // Animation_PlayLoop(object_tite_Anim_000C70).
        self.change(|a| &a.jump, 1.0, ANIMMODE_LOOP, 0.0);
        self.action = TEKTITE_MOVE_TOWARD_PLAYER;
        self.actor.velocity.y = 10.0;
        self.actor.gravity = -1.0;
        self.actor.speed_xz = 4.0;
        // vQueuedJumps (a u8).
        self.action_var2 = play.rand.s16_offset(1, 3) as u8;
        self.jump_sfx(play);
        self.func = Func::MoveTowardPlayer;
    }

    /// Another hop (`EnTite_MoveTowardPlayer`).
    fn hop(&mut self, play: &mut PlayState) {
        self.actor.velocity.y = 10.0;
        self.actor.speed_xz = 4.0;
        self.actor.flags |= ACTOR_FLAG_SFX_FOR_PLAYER_BODY_HIT;
        self.actor.gravity = -1.0;
        self.jump_sfx(play);
    }

    /// `EnTite_MoveTowardPlayer`: hops toward Link as a way to travel (no hurting).
    fn move_toward_player(&mut self, play: &mut PlayState) {
        smooth_step_to_f(&mut self.actor.speed_xz, 0.0, 0.1, 1.0, 0.0);
        self.skel.update();

        if self.actor.bg_check_flags & (BGCHECKFLAG_GROUND_TOUCH | BGCHECKFLAG_WATER_TOUCH) != 0 {
            if self.actor.bg_check_flags & BGCHECKFLAG_WATER_TOUCH == 0 {
                self.feet_dust(play);
                audio_play_actor_sfx2(play, NA_SE_EN_DODO_M_GND);
            } else {
                audio_play_actor_sfx2(play, NA_SE_EN_TEKU_LAND_WATER);
            }
        }

        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND_TOUCH != 0 || (self.blue() && self.actor.bg_check_flags & BGCHECKFLAG_WATER_TOUCH != 0) {
            // vQueuedJumps.
            if self.action_var2 != 0 {
                self.action_var2 -= 1;
            } else {
                self.setup_idle(play);
            }
        }

        if (self.actor.bg_check_flags & (BGCHECKFLAG_GROUND | BGCHECKFLAG_GROUND_TOUCH) != 0
            || (self.blue() && self.actor.bg_check_flags & (BGCHECKFLAG_WATER | BGCHECKFLAG_WATER_TOUCH) != 0))
            && self.actor.velocity.y <= 0.0
        {
            // Landed: turn a little to Link and snap to the ground or the water.
            self.actor.speed_xz = 0.0;
            smooth_step_to_s(&mut self.actor.shape_rot.y, self.actor.yaw_towards_player, 1, 4000, 0);
            self.actor.world_rot.y = self.actor.shape_rot.y;
            if !self.blue_in_water() {
                if self.actor.floor_height > BGCHECK_Y_MIN {
                    self.actor.world_pos.y = self.actor.floor_height;
                }
            } else if self.actor.bg_check_flags & BGCHECKFLAG_WATER_TOUCH != 0 {
                let mut ripple_pos = self.actor.world_pos;
                self.actor.bg_check_flags &= !BGCHECKFLAG_WATER_TOUCH;
                ripple_pos.y += self.actor.y_dist_to_water;
                self.actor.gravity = 0.0;
                self.actor.velocity.y *= 0.75;
                Self::effect_ss_g_ripple_spawn(ripple_pos, 0, 500, 0);
                return;
            } else {
                // Submerged: float to the surface.
                smooth_step_to_f(&mut self.actor.velocity.y, 0.0, 1.0, 2.0, 0.0);
                let target = self.actor.world_pos.y + self.actor.y_dist_to_water;
                smooth_step_to_f(&mut self.actor.world_pos.y, target, 1.0, 2.0, 0.0);
                if self.actor.y_dist_to_water != 0.0 {
                    // Not till it's at the surface.
                    return;
                }
            }

            // Idle or turn with Link far, else hop on.
            if self.player_far() {
                self.setup_idle(play);
            } else if self.actor.xz_dist_to_player <= 180.0 && self.actor.y_dist_to_player <= 80.0 {
                // vQueuedJumps <= 0 (a u8: 0).
                if self.action_var2 == 0 {
                    self.setup_turn_toward_player();
                } else {
                    self.hop(play);
                }
            } else {
                self.hop(play);
            }
        } else {
            // In the air: turn slowly to Link.
            self.actor.flags |= ACTOR_FLAG_SFX_FOR_PLAYER_BODY_HIT;
            smooth_step_to_s(&mut self.actor.shape_rot.y, self.actor.yaw_towards_player, 1, 1000, 0);
            if self.actor.velocity.y >= 6.0 && self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
                self.feet_sparkles(play);
            }
        }
    }

    /// `EnTite_SetupRecoil`.
    fn setup_recoil(&mut self) {
        self.action = TEKTITE_RECOIL;
        // Animation_MorphToLoop(object_tite_Anim_0012E4, 4).
        self.change(|a| &a.idle, 1.0, ANIMMODE_LOOP, 4.0);
        self.actor.speed_xz = -6.0;
        self.actor.world_rot.y = self.actor.yaw_towards_player;
        self.actor.gravity = -1.0;
        self.func = Func::Recoil;
    }

    /// The snap to the ground or the water's surface on landing (`EnTite_Recoil`,
    /// `EnTite_Stunned`).
    fn snap_on_landing(&mut self) {
        if self.grounded_or_blue_in_water() && self.actor.velocity.y <= 0.0 {
            if !self.blue_in_water() {
                if self.actor.floor_height > BGCHECK_Y_MIN {
                    self.actor.world_pos.y = self.actor.floor_height;
                }
            } else {
                self.actor.velocity.y = 0.0;
                self.actor.gravity = 0.0;
                self.actor.world_pos.y += self.actor.y_dist_to_water;
            }
        }
    }

    /// The landing's sound and dust (`EnTite_Recoil`, `EnTite_Stunned`).
    fn landing_sfx(&mut self, play: &mut PlayState) {
        if self.actor.bg_check_flags & (BGCHECKFLAG_GROUND_TOUCH | BGCHECKFLAG_WATER_TOUCH) != 0 {
            if self.actor.bg_check_flags & BGCHECKFLAG_WATER_TOUCH == 0 {
                self.feet_dust(play);
                audio_play_actor_sfx2(play, NA_SE_EN_DODO_M_GND);
            } else {
                self.actor.bg_check_flags &= !BGCHECKFLAG_WATER_TOUCH;
                audio_play_actor_sfx2(play, NA_SE_EN_TEKU_LAND_WATER2);
            }
        }
    }

    /// On the ground, or the blue one in water (`BGCHECKFLAG_GROUND` alone).
    fn standing(&self) -> bool {
        self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 || self.blue_in_water()
    }

    /// Recoil's and Stunned's choice once stopped: idle with Link far (and it upright), attack
    /// within 180 facing him (6000), else hop at him.
    fn choose_after_recoil(&mut self, play: &mut PlayState, angle_to_player: i16) {
        if self.player_far() && abs_i(self.actor.shape_rot.x) < 4000 && abs_i(self.actor.shape_rot.z) < 4000 && self.standing() {
            self.setup_idle(play);
        } else if self.actor.xz_dist_to_player < 180.0 && self.actor.y_dist_to_player <= 80.0 && abs_i(angle_to_player) <= 6000 {
            self.setup_attack(play);
        } else {
            self.setup_move_toward_player(play);
        }
    }

    /// `EnTite_Recoil`: back off a hit (given or taken), sliding to a stop on landing.
    fn recoil(&mut self, play: &mut PlayState) {
        smooth_step_to_f(&mut self.actor.speed_xz, 0.0, 1.0, 0.5, 0.0);
        self.snap_on_landing();
        self.landing_sfx(play);
        let angle_to_player = self.actor.yaw_towards_player.wrapping_sub(self.actor.shape_rot.y);
        if self.actor.speed_xz == 0.0 && self.standing() {
            self.actor.world_rot.y = self.actor.shape_rot.y;
            self.collider.base.at_flags &= !AT_HIT;
            self.choose_after_recoil(play, angle_to_player);
        }
        self.skel.update();
    }

    /// `EnTite_SetupStunned`: held on the idle's first frame, knocked back 6.
    fn setup_stunned(&mut self, play: &mut PlayState) {
        // Animation_Change(object_tite_Anim_0012E4, 0, 0, last, ANIMMODE_LOOP, 4).
        if let Some(a) = self.anim(|a| &a.idle) {
            let last = a.last_frame();
            self.skel.change(a, 0.0, 0.0, last, ANIMMODE_LOOP, 4.0);
        }
        self.action = TEKTITE_STUNNED;
        self.actor.speed_xz = -6.0;
        self.actor.world_rot.y = self.actor.yaw_towards_player;
        if self.damage_reaction == 0xF {
            self.spawn_ice_timer = 48;
        }
        audio_play_actor_sfx2(play, NA_SE_EN_GOMA_JR_FREEZE);
        self.func = Func::Stunned;
    }

    /// `EnTite_Stunned`: stunned or frozen till its colour's gone, stopped and down.
    fn stunned(&mut self, play: &mut PlayState) {
        smooth_step_to_f(&mut self.actor.speed_xz, 0.0, 1.0, 0.5, 0.0);
        self.snap_on_landing();
        self.landing_sfx(play);
        let angle_to_player = self.actor.yaw_towards_player.wrapping_sub(self.actor.shape_rot.y);
        if self.actor.color_filter_timer == 0 && self.actor.speed_xz == 0.0 && self.standing() {
            self.actor.world_rot.y = self.actor.shape_rot.y;
            if self.actor.col_chk_info.health == 0 {
                self.setup_death_cry();
            } else if self.flip_state == TEKTITE_FLIPPED {
                self.setup_flip_upright(play);
            } else {
                self.choose_after_recoil(play, angle_to_player);
            }
        }
        self.skel.update();
    }

    /// `EnTite_SetupDeathCry`.
    fn setup_death_cry(&mut self) {
        self.action = TEKTITE_DEATH_CRY;
        self.actor.color_filter_timer = 0;
        self.actor.speed_xz = 0.0;
        self.func = Func::DeathCry;
    }

    /// `EnTite_DeathCry`: its cry, and room for its parts (`BodyBreak_Alloc`, 24).
    fn death_cry(&mut self, play: &mut PlayState) {
        let pos = self.actor.projected_pos;
        play.with_ss(|s| s.dead_sound_spawn_stationary(pos, NA_SE_EN_TEKU_DEAD, 1, DEADSOUND_REPEAT_MODE_OFF, 40));
        self.action = TEKTITE_FALL_APART;
        self.func = Func::FallApart;
        self.body_break.alloc(24);
    }

    /// `EnTite_FallApart`: once drawn, its parts (`En_Part` of kind `params + 0xB`) and a drop.
    fn fall_apart(&mut self, play: &mut PlayState) {
        // schema: the parts' kind is the params plus 0xB (10 red, 9 blue)
        let kind = self.actor.params.wrapping_add(0xB);
        if self.body_break.spawn_parts(&mut self.actor, play, kind) {
            let pos = self.actor.world_pos;
            let table = if self.blue() { DROP_TABLE_14 } else { DROP_TABLE_4 };
            crate::en_item00::item_drop_collectible_random(play, Some(&self.actor), pos, table);
            self.actor.kill();
        }
    }

    /// `EnTite_SetupFlipOnBack`.
    fn setup_flip_on_back(&mut self, play: &mut PlayState) {
        // Animation_PlayLoopSetSpeed(object_tite_Anim_000A14, 1.5).
        self.change(|a| &a.turn, 1.5, ANIMMODE_LOOP, 0.0);
        audio_play_actor_sfx2(play, NA_SE_EN_TEKU_REVERSE);
        self.flip_state = TEKTITE_FLIPPED;
        // vOnBackTimer.
        self.action_var1 = 500;
        self.actor.speed_xz = 0.0;
        self.actor.gravity = -1.0;
        // vLegTwitchTimer (a u8).
        self.action_var2 = (play.rand.zero_one() * 50.0) as u8;
        self.actor.velocity.y = 11.0;
        self.func = Func::FlipOnBack;
    }

    /// `EnTite_FlipOnBack`: the flip, then on its back twitching its legs.
    fn flip_on_back(&mut self, play: &mut PlayState) {
        smooth_step_to_s(&mut self.actor.shape_rot.z, 0x7FFF, 1, 4000, 0);
        // The legs' animation restarted at random: twitching. (A u8: from 0 it wraps to 255.)
        self.action_var2 = self.action_var2.wrapping_sub(1);
        if self.action_var2 == 0 {
            self.action_var2 = (play.rand.zero_one() * 30.0) as u8;
            self.skel.cur_frame = play.rand.zero_one() * 5.0;
        }
        self.skel.update();
        if self.actor.bg_check_flags & (BGCHECKFLAG_GROUND | BGCHECKFLAG_GROUND_TOUCH) != 0 {
            // Landed: dust and noise.
            if self.actor.bg_check_flags & BGCHECKFLAG_GROUND_TOUCH != 0 {
                let pos = self.actor.world_pos;
                actor_spawn_floor_dust_ring(play, &self.actor, pos, 20.0, 11, 4.0, 0, 0, false);
                audio_play_actor_sfx2(play, NA_SE_EN_DODO_M_GND);
            }
            // vOnBackTimer.
            self.action_var1 -= 1;
            if self.action_var1 == 0 {
                self.setup_flip_upright(play);
            }
        } else if self.actor.shape_y_offset < 2800.0 {
            // Its position from its feet to its back during the flip.
            self.actor.shape_y_offset += 400.0;
        }
    }

    /// `EnTite_SetupFlipUpright`. @bug (game): on water (a blue one) gravity is 0, so its 13 up
    /// never falls: the "flying Tektite".
    fn setup_flip_upright(&mut self, play: &mut PlayState) {
        self.flip_state = TEKTITE_UNFLIPPED;
        // Unused here; SetupIdle overwrites it.
        self.action_var1 = 1000;
        self.actor.velocity.y = 13.0;
        audio_play_actor_sfx2(play, NA_SE_EN_TEKU_REVERSE);
        self.func = Func::FlipUpright;
    }

    /// `EnTite_FlipUpright`. @bug (game): the flying Tektite never touches the ground and stays
    /// here for good.
    fn flip_upright(&mut self, play: &mut PlayState) {
        smooth_step_to_s(&mut self.actor.shape_rot.z, 0, 1, 0xFA0, 0);
        self.skel.update();
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND_TOUCH != 0 {
            self.feet_dust(play);
            self.actor.shape_y_offset = 0.0;
            self.actor.world_pos.y = self.actor.floor_height;
            audio_play_actor_sfx2(play, NA_SE_EN_DODO_M_GND);
            self.setup_idle(play);
        }
    }

    /// `EnTite_CheckDamage`.
    fn check_damage(&mut self, play: &mut PlayState) {
        if self.collider.base.ac_flags & AC_HIT != 0 && self.action >= TEKTITE_IDLE {
            self.collider.base.ac_flags &= !AC_HIT;
            // Fire magic does nothing.
            if self.actor.col_chk_info.damage_reaction != 0xE {
                self.damage_reaction = self.actor.col_chk_info.damage_reaction;
                let elem = self.collider.elements[0].info;
                self.actor.set_drop_flag(&elem, false);
                // Stunned by a nut, the boomerang, the hookshot, an ice arrow or ice magic.
                if self.actor.col_chk_info.damage_reaction == 1 || self.actor.col_chk_info.damage_reaction == 0xF {
                    if self.action != TEKTITE_STUNNED {
                        self.actor.set_color_filter(COLORFILTER_COLORFLAG_BLUE, 120, COLORFILTER_BUFFLAG_OPA, 80);
                        self.actor.apply_damage();
                        self.setup_stunned(play);
                    }
                } else {
                    // Damage, and the death.
                    if self.actor.color_filter_timer == 0 || self.actor.color_filter_params & 0x4000 == 0 {
                        self.actor.set_color_filter(COLORFILTER_COLORFLAG_RED, 255, COLORFILTER_BUFFLAG_OPA, 8);
                        self.actor.apply_damage();
                    }
                    if self.actor.col_chk_info.health == 0 {
                        self.setup_death_cry();
                    } else {
                        // Flipped back up if it's on its back.
                        audio_play_actor_sfx2(play, NA_SE_EN_TEKU_DAMAGE);
                        if self.flip_state != TEKTITE_FLIPPED {
                            self.setup_recoil();
                        } else {
                            self.setup_flip_upright(play);
                        }
                    }
                }
            }
        } else if self.actor.col_chk_info.health != 0 && play.actors.unk_02 != 0 && self.actor.xz_dist_to_player <= 400.0 && self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
            // The hammer's floor hit with Link near: over it goes (or back up).
            if self.flip_state == TEKTITE_FLIPPED {
                self.setup_flip_upright(play);
            } else if self.action >= TEKTITE_IDLE || self.action >= TEKTITE_IDLE {
                self.setup_flip_on_back(play);
            }
        }
    }

    fn run_action(&mut self, play: &mut PlayState) {
        match self.func {
            Func::Idle => self.idle(play),
            Func::Attack => self.attack(play),
            Func::TurnTowardPlayer => self.turn_toward_player(play),
            Func::MoveTowardPlayer => self.move_toward_player(play),
            Func::Recoil => self.recoil(play),
            Func::Stunned => self.stunned(play),
            Func::DeathCry => self.death_cry(play),
            Func::FallApart => self.fall_apart(play),
            Func::FlipOnBack => self.flip_on_back(play),
            Func::FlipUpright => self.flip_upright(play),
        }
    }

    /// A foot on the water (`EnTite_Update`): every 8 frames (`gameplayFrames + phase`) or every
    /// frame falling, a foot at or under the surface is put on it and ripples.
    fn foot_on_water(play: &PlayState, foot: &mut Vec3, phase: u32, falling: bool) {
        if (play.gameplay_frames.wrapping_add(phase) % 8 == 0 || falling)
            && let Some(water_surface_y) = play.col.water_surface(foot.x, foot.z, play.col.water_room)
            && foot.y <= water_surface_y
        {
            foot.y = water_surface_y;
            Self::effect_ss_g_ripple_spawn(*foot, 0, 220, 0);
        }
    }

    /// The limbs' lists (1-based), from the skeleton's table (`object_tite_DL_*` by address).
    fn limb_dlist(skeleton: &Skeleton, limb: usize) -> Option<DList> {
        let addr = *skeleton.limbs.get(limb.checked_sub(1)?)?.dlists.first()?;
        if addr == 0 {
            return None;
        }
        let name = format!("object_tite_DL_{:06X}", addr & 0x00FF_FFFF);
        DLISTS.iter().find(|&&s| s == name).map(|&s| (OBJECT, s))
    }
}

impl ActorImpl for EnTite {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnTite_Update`.
    fn update(&mut self, play: &mut PlayState) {
        self.check_damage(play);
        // Still this frame for an immune hit (fire magic).
        if self.actor.col_chk_info.damage_reaction != 0xE {
            self.run_action(play);
            self.actor.move_forward();
            self.actor.update_bg_check_info(&play.col, 25.0, 40.0, 20.0, self.unk_2dc);
            // On water: the feet on the surface, rippling. (The floor poly saved and put back
            // round BgCheck_GetWaterSurface, which doesn't touch it here.)
            if self.blue_in_water() {
                let falling = self.actor.velocity.y < 0.0;
                Self::foot_on_water(play, &mut self.back_right_foot_pos, 0, falling);
                Self::foot_on_water(play, &mut self.back_left_foot_pos, 2, falling);
                Self::foot_on_water(play, &mut self.front_left_foot_pos, 4, falling);
                Self::foot_on_water(play, &mut self.front_right_foot_pos, 1, falling);
            }

            if self.actor.bg_check_flags & (BGCHECKFLAG_GROUND | BGCHECKFLAG_GROUND_TOUCH) != 0 {
                // On the ground: along its slope, upside down if flipped.
                let mut rot = self.actor.shape_rot;
                func_800359b8(&self.actor, play, self.actor.shape_rot.y, &mut rot);
                self.actor.shape_rot = rot;
                if self.flip_state >= TEKTITE_FLIPPED {
                    self.actor.shape_rot.z = self.actor.shape_rot.z.wrapping_add(0x7FFF);
                }
            } else {
                // Else turning back upright.
                smooth_step_to_s(&mut self.actor.shape_rot.x, 0, 1, 1000, 0);
                if self.flip_state <= TEKTITE_UNFLIPPED {
                    smooth_step_to_s(&mut self.actor.shape_rot.z, 0, 1, 1000, 0);
                    if self.actor.shape_y_offset > 0.0 {
                        self.actor.shape_y_offset -= 400.0;
                    }
                }
            }
        }
        self.actor.focus_pos = self.actor.world_pos;
        self.actor.focus_pos.y += 20.0;

        play.collision_check_set_ac(&self.actor, 0, &mut self.collider);
        play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
    }

    /// `EnTite_Draw`'s changes: its sphere on the actor's matrix (`Collider_UpdateSpheres(0)`),
    /// `EnTite_PostLimbDraw` (the feet 2800 along limbs 8, 13, 18 and 23; every limb's list to
    /// the body break), then while frozen the ice: its colour held on, a chunk every 4 frames.
    fn draw_update(&mut self, play: &mut PlayState) {
        let model = actor_draw_matrix(&RenderState::of(&self.actor));
        self.collider.update_spheres(0, &model);
        if let Some(skeleton) = self.skeleton.clone() {
            let freeze = play.actors.freeze_flash_timer;
            let joints = self.skel.joint_table.clone();
            let mut posts = Vec::new();
            draw_opa_pose(&skeleton, &joints, |_, _, _| LimbDraw::Default(Mat4::IDENTITY), |limb, m| posts.push((limb, m)));
            for (limb, m) in posts {
                let m = model * m;
                match limb {
                    LIMB_BACK_RIGHT_FOOT => self.back_right_foot_pos = m.transform_point3(FOOT_OFFSET),
                    LIMB_FRONT_RIGHT_FOOT => self.front_right_foot_pos = m.transform_point3(FOOT_OFFSET),
                    LIMB_BACK_LEFT_FOOT => self.back_left_foot_pos = m.transform_point3(FOOT_OFFSET),
                    LIMB_FRONT_LEFT_FOOT => self.front_left_foot_pos = m.transform_point3(FOOT_OFFSET),
                    _ => {}
                }
                let dlist = Self::limb_dlist(&skeleton, limb);
                self.body_break.set_info(freeze, limb as i32, 0, 24, 24, dlist, BODYBREAK_OBJECT_SLOT_DEFAULT, m);
            }
        }

        if self.spawn_ice_timer != 0 {
            // Chunks of ice all over its body.
            self.actor.color_filter_timer = self.actor.color_filter_timer.wrapping_add(1);
            self.spawn_ice_timer -= 1;
            if self.spawn_ice_timer & 3 == 0 {
                let idx = (self.spawn_ice_timer >> 2) as usize;
                let ice_chunk = self.actor.world_pos + ICE_CHUNKS[idx];
                let r = self.actor.shape_rot;
                let me = play.cur_actor.map(|h| oot_game::effect::SsActor { handle: h, world_pos: self.actor.world_pos, shape_rot: [r.x, r.y, r.z] });
                play.effect_ss_en_ice_spawn_flying_vec3f(me, ice_chunk, [150, 150, 150, 250], [235, 245, 255], 1.0);
            }
        }
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        rs.values = vec![self.shadow_scale * self.actor.scale.x * 100.0];
        rs.switches = vec![self.blue() as u32];
        rs
    }

    /// `EnTite_Draw`: the skeleton with the red or the blue textures; the circle shadow
    /// (`ActorShadow_DrawCircle`) under it.
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let (Some(joints), Some(skeleton)) = (&rs.joints, &self.skeleton) else { return };
        let ([shadow], [blue]) = (rs.values.as_slice(), rs.switches.as_slice()) else { return };
        if *shadow > 0.0 {
            // ActorShadow_DrawCircle (shadowScale × scale): the circle shadow's stand-in.
            let (floor, _) = play.col.entity_raycast_down(rs.pos + Vec3::Y * 20.0);
            let m = Mat4::from_translation(Vec3::new(rs.pos.x, floor + 0.3, rs.pos.z)) * Mat4::from_scale(Vec3::new(*shadow, 1.0, *shadow));
            out.xlu.push(DrawCmd::new(MeshKey::named(oot_game::play::builtin::SHADOW), m));
        }
        let bones = skeleton.pose(joints);
        let mesh = MeshKey::named(keys::bake(if *blue != 0 { BAKE_BLUE } else { BAKE_RED }));
        out.opa.push(DrawCmd { mesh, transform: actor_draw_matrix(rs), bones, params: Default::default() });
    }

    /// `EnTite_Destroy`: its spawner (`En_Encount1`) counted down.
    fn destroy(&mut self, play: &mut PlayState) {
        if let Some(p) = self.actor.parent.and_then(|h| play.actors.downcast_mut::<crate::en_encount1::EnEncount1>(h))
            && !p.actor.killed
            && p.cur_num_spawn > 0
        {
            p.cur_num_spawn -= 1;
            log::debug!("En_Tite: Number of simultaneous spawns {}", p.cur_num_spawn);
        }
    }

    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        (id == 0).then_some(ColliderMut::JntSph(&mut self.collider))
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
