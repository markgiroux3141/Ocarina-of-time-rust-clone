//! `En_Am` (`ovl_En_Am/z_en_am.c`): the Armos, a stone soldier on a pedestal (GAME-09 milestone 3).
//! Two kinds (`ArmosType`): the enemy, asleep as a statue until Link touches or hits it, and the
//! statue (params 0), a DynaPoly actor Link pushes like a block. Dodongo's Cavern's room 8 (Master
//! Quest) places seven enemies (params -1); the Spirit Temple and Ganon's Castle's trials place
//! more.
//!
//! - **Asleep** (`EnAm_Sleep`): its textures the statue's (`textureBlend` 0). Link bumping into it
//!   (its hurt cylinder's `OC1_HIT` on him) or a hit that isn't turned away by its metal wakes it:
//!   `NA_SE_EN_AMOS_WAVE`, `NA_SE_EN_AMOS_VOICE`, a red flash, then 12 frames shaking about where it
//!   stood while its textures turn (20 a frame); at 240 it's awake (`attackTimer` 200).
//! - **The chase**: three hops at Link (`EnAm_Lunge`: 12 up and 6 on at the hop's frame 8 when the
//!   floor 80 ahead is one it can stand on, `EnAm_CanMove`; its sword's quad hurting 8 in the air),
//!   turning to him on each landing, then 40 frames' rest (`EnAm_Cooldown`) and three more. Over
//!   240 from home or `attackTimer` out, it hops home (`EnAm_RotateToHome`, `_MoveToHome`,
//!   `_RotateToInit`) and sleeps again, its textures turning back.
//! - **The hits** (`EnAm_UpdateDamage`, awake): the block cylinder (`AC_HARD`, metal) turns away the
//!   Deku Stick, the slingshot and the Kokiri Sword's slash and spin (`EnAm_Ricochet`: knocked back
//!   6); a Deku Nut, the boomerang or the hookshot stuns it (`EnAm_Stunned`: blue 100 frames, ice
//!   arrows and ice magic freeze it); stunned, any hit kills it, as does any `AM_DMG_REACT_KILL`
//!   (explosives, arrows, the Master Sword, the hammer); anything else (the Kokiri Sword's jump)
//!   ricochets. Its sword bounced off Link's shield: a ricochet too.
//! - **The death** (`EnAm_SetupRecoilFromDamage`): knocked back (`NA_SE_EN_AMOS_DAMAGE`, the
//!   finishing blow's freeze), then 64 frames: 12 still, then hopping about spinning faster
//!   (`panicSpinRot`) and flashing red every 4; at 0 its own `En_Bom` with no fuse (`shape.rot.z`
//!   2), `NA_SE_EN_AMOS_DEAD`, a drop from table 10 and ten puffs of grey dust, and it's gone. The
//!   blast (ADR 0072) hurts Link and kills other Armos that are awake (it wakes sleeping ones).
//! - **The statue** (`EnAm_Statue`): `gArmosCol` in `ACTORCAT_BG`, not targetable. Player's push
//!   (`dyna.unk_150`, `unk_158`) slides it in 16-frame steps (`unk_258` from 0x8000 down by 0x800,
//!   speed `sin(unk_258)` for Player's push of 2: about 15 a step), `NA_SE_EV_ROCK_SLIDE`; a step
//!   stops early off the ground, at a wall ahead (`func_800435D8`) or against an actor ahead (its
//!   hurt cylinder's OC); each stop clears Player's push flags and speed.
//!
//! The whole overlay is ported. The draw (`EnAm_Draw`) is `gArmosSkel` with `textureBlend` as the
//! env alpha (the statue's and the enemy's textures blended), its colour filter Actor_Draw's; the
//! circle shadow (48) the shape's (`ActorShape_Init`'s, drawn by `Actor_Draw`). `draw_update` puts
//! the sword's quad on limb 1
//! (`EnAm_PostLimbDraw` → `EnAm_TransformSwordHitbox`) while it lunges and spawns the ice chunks
//! while it's frozen (`EffectSsEnIce`).
//!
//! Params: `params` whole, the type (`ArmosType`: 0 `ARMOS_STATUE`, pushed; any other an enemy,
//! `ARMOS_ENEMY` 1, -1 as Dodongo's Cavern places them).
//! Flags: none.

use std::sync::Arc;

use eng_anim::skeleton::Skeleton;
use eng_collision::dyna::{BG_ACTOR_MAX, BgActorSource};
use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use eng_math::{cos_s, sin_s, smooth_step_to_f, smooth_step_to_s, vec3f_yaw};
use glam::{Mat4, Vec3};
use oot_game::actor::*;
use oot_game::actor_ctx::{
    ACTORCAT_BG, ACTORCAT_ENEMY, ActorImpl, ActorProfile, actor_spawn_floor_dust_ring, actor_test_floor_in_direction, audio_play_actor_sfx2, audio_play_actor_sfx_at, enemy_start_finishing_blow,
};
use oot_game::actor_schema::{ActorSchema, Field, Kind};
use oot_game::audio::sfx::{NA_SE_EN_GOMA_JR_FREEZE, NA_SE_EV_ROCK_SLIDE, NA_SE_IT_SHIELD_REFLECT_SW, NA_SE_PL_BODY_HIT, SFX_FLAG, SfxPos};
use oot_game::collision_check::*;
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::skelanime_std::{ANIMMODE_LOOP, ANIMMODE_ONCE, Anim, LimbDraw, SkelAnimeStd, draw_opa_pose};

use crate::obj_oshihiki::NA_SE_EV_BLOCK_BOUND;

/// `ACTOR_EN_AM` (`actor_table.h`: 0x0054).
pub const ACTOR_EN_AM: i16 = 0x0054;
/// `OBJECT_AM` (`object_table.h`: 0x0038).
pub const OBJECT: &str = "object_am";
const SKEL: &str = "gArmosSkel";
const COLLISION: &str = "gArmosCol";

/// `En_Am_Profile`: `ACTORCAT_ENEMY`, attention-enabled, hostile, always updating, pressing
/// switches.
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_EN_AM,
    name: "En_Am",
    category: ACTORCAT_ENEMY,
    flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE | ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_CAN_PRESS_SWITCHES,
    object: OBJECT,
    schema: &SCHEMA,
};

/// `this->dyna.actor.params`: the type (`ArmosType`; the C only tells `ARMOS_STATUE` from the rest).
pub const TYPE: Field = Field::whole(
    "params",
    Kind::Named(&[(0, "ARMOS_STATUE"), (1, "ARMOS_ENEMY"), (-1, "ARMOS_ENEMY (as Dodongo's Cavern places it: any value but 0)")]),
    "a statue Link pushes (0), or an enemy asleep as a statue (any other)",
);

/// En_Am's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema { fields: &[TYPE], flags: &[], todo: false, objects: &[] };

/// `ArmosType` (`z_en_am.h`).
pub const ARMOS_STATUE: i16 = 0;
pub const ARMOS_ENEMY: i16 = 1;

/// `ArmosBehavior`.
pub const AM_BEHAVIOR_NONE: i32 = 0;
pub const AM_BEHAVIOR_DAMAGED: i32 = 1;
pub const AM_BEHAVIOR_DO_NOTHING: i32 = 3;
/// Checked but never set.
pub const AM_BEHAVIOR_5: i32 = 5;
pub const AM_BEHAVIOR_STUNNED: i32 = 6;
pub const AM_BEHAVIOR_GO_HOME: i32 = 7;
pub const AM_BEHAVIOR_RICOCHET: i32 = 8;
pub const AM_BEHAVIOR_AGGRO: i32 = 10;

/// `ArmosDamageReaction`.
pub const AM_DMG_REACT_NONE: u8 = 0;
pub const AM_DMG_REACT_NUT: u8 = 1;
pub const AM_DMG_REACT_STUN: u8 = 6;
pub const AM_DMG_REACT_ICE: u8 = 13;
pub const AM_DMG_REACT_MAGIC_FIRE_LIGHT: u8 = 14;
pub const AM_DMG_REACT_KILL: u8 = 15;

// The sounds (`include/tables/sfx/enemybank_table.h`).
/// `NA_SE_EN_AMOS_WALK` (0x3844).
pub const NA_SE_EN_AMOS_WALK: u16 = 0x3844;
/// `NA_SE_EN_AMOS_WAVE` (0x3845).
pub const NA_SE_EN_AMOS_WAVE: u16 = 0x3845;
/// `NA_SE_EN_AMOS_DEAD` (0x3846).
pub const NA_SE_EN_AMOS_DEAD: u16 = 0x3846;
/// `NA_SE_EN_AMOS_DAMAGE` (0x3847).
pub const NA_SE_EN_AMOS_DAMAGE: u16 = 0x3847;
/// `NA_SE_EN_AMOS_VOICE` (0x3848).
pub const NA_SE_EN_AMOS_VOICE: u16 = 0x3848;

/// `NAVI_ENEMY_ARMOS` (`actor.h`: 0x13).
pub const NAVI_ENEMY_ARMOS: u8 = 0x13;

/// `PLAYER_STATE2_0`, `_4`, `_6`, `_8` (`player.h`): the push's (grabbing, pushing, the push
/// camera).
const PLAYER_STATE2_0: u32 = 1 << 0;
const PLAYER_STATE2_4: u32 = 1 << 4;
const PLAYER_STATE2_6: u32 = 1 << 6;
const PLAYER_STATE2_8: u32 = 1 << 8;

/// `BOMB_BODY` (`z_en_bom.h`).
const BOMB_BODY: i16 = 0;
/// `COLLECTIBLE_DROP_RANDOM_PARAMS(COLLECTIBLE_DROP_TABLE_10, false)` (`z_en_item00.h`: the table
/// × 16).
const DROP_TABLE_10: i16 = 10 * 16;

/// `sHurtCylinderInit`: its body, hit by Player's 0xFFCFFFFF, pushing and pushed.
const HURT_CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_HIT5, at_flags: AT_NONE, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 15, height: 70, y_shift: 0, pos: [0, 0, 0] },
};

/// `sBlockCylinderInit`: its metal, turning away (`AC_HARD`) the Deku Stick, the slingshot and the
/// Kokiri Sword's slash and spin (0x00400106).
const BLOCK_CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_METAL, at_flags: AT_NONE, ac_flags: AC_ON | AC_HARD | AC_TYPE_PLAYER, oc_flags1: OC1_NONE, oc_flags2: OC2_NONE, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0x0040_0106, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_NONE,
    },
    dim: Cylinder16 { radius: 15, height: 70, y_shift: 0, pos: [0, 0, 0] },
};

/// `sQuadInit`: its sword, hurting 8.
const QUAD_INIT: ColliderQuadInit = ColliderQuadInit {
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

/// `sDamageTable`.
pub static S_DAMAGE_TABLE: DamageTable = DamageTable {
    table: [
        dmg_entry(0, AM_DMG_REACT_NUT),              // Deku nut
        dmg_entry(2, AM_DMG_REACT_NONE),             // Deku stick
        dmg_entry(1, AM_DMG_REACT_NONE),             // Slingshot
        dmg_entry(2, AM_DMG_REACT_KILL),             // Explosive
        dmg_entry(0, AM_DMG_REACT_STUN),             // Boomerang
        dmg_entry(2, AM_DMG_REACT_KILL),             // Normal arrow
        dmg_entry(2, AM_DMG_REACT_KILL),             // Hammer swing
        dmg_entry(0, AM_DMG_REACT_STUN),             // Hookshot
        dmg_entry(1, AM_DMG_REACT_NONE),             // Kokiri sword
        dmg_entry(2, AM_DMG_REACT_KILL),             // Master sword
        dmg_entry(4, AM_DMG_REACT_KILL),             // Giant's Knife
        dmg_entry(2, AM_DMG_REACT_KILL),             // Fire arrow
        dmg_entry(4, AM_DMG_REACT_ICE),              // Ice arrow
        dmg_entry(2, AM_DMG_REACT_KILL),             // Light arrow
        dmg_entry(2, AM_DMG_REACT_NONE),             // Unk arrow 1
        dmg_entry(2, AM_DMG_REACT_NONE),             // Unk arrow 2
        dmg_entry(2, AM_DMG_REACT_NONE),             // Unk arrow 3
        dmg_entry(0, AM_DMG_REACT_MAGIC_FIRE_LIGHT), // Fire magic
        dmg_entry(3, AM_DMG_REACT_ICE),              // Ice magic
        dmg_entry(0, AM_DMG_REACT_MAGIC_FIRE_LIGHT), // Light magic
        dmg_entry(0, AM_DMG_REACT_NONE),             // Shield
        dmg_entry(0, AM_DMG_REACT_NONE),             // Mirror Ray
        dmg_entry(1, AM_DMG_REACT_NONE),             // Kokiri spin
        dmg_entry(4, AM_DMG_REACT_KILL),             // Giant spin
        dmg_entry(2, AM_DMG_REACT_KILL),             // Master spin
        dmg_entry(2, AM_DMG_REACT_NONE),             // Kokiri jump
        dmg_entry(8, AM_DMG_REACT_KILL),             // Giant jump
        dmg_entry(4, AM_DMG_REACT_KILL),             // Master jump
        dmg_entry(0, AM_DMG_REACT_NONE),             // Unknown 1
        dmg_entry(0, AM_DMG_REACT_NONE),             // Unblockable
        dmg_entry(4, AM_DMG_REACT_KILL),             // Hammer jump
        dmg_entry(0, AM_DMG_REACT_NONE),             // Unknown 2
    ],
};

/// `EnAm_TransformSwordHitbox`'s corners on limb 1: `D_809B0080`, `D_809B0074`, `D_809B0098`,
/// `D_809B008C` (the quad's 0 to 3).
const SWORD_QUAD: [Vec3; 4] = [Vec3::new(-2500.0, 0.0, 0.0), Vec3::new(2500.0, 7000.0, 0.0), Vec3::new(-2500.0, 0.0, 4000.0), Vec3::new(2500.0, 7000.0, 4000.0)];
/// The limb `EnAm_PostLimbDraw` takes the sword from (1-based).
const LIMB_SWORD: usize = 1;

/// `sIcePosOffsets`: where the ice chunks go over a frozen Armos.
const ICE_POS_OFFSETS: [Vec3; 12] = [
    Vec3::new(20.0, 40.0, 0.0),
    Vec3::new(10.0, 60.0, 10.0),
    Vec3::new(-10.0, 60.0, 10.0),
    Vec3::new(-20.0, 40.0, 0.0),
    Vec3::new(10.0, 60.0, -10.0),
    Vec3::new(-10.0, 60.0, -10.0),
    Vec3::new(0.0, 40.0, -20.0),
    Vec3::new(10.0, 20.0, 10.0),
    Vec3::new(10.0, 20.0, -10.0),
    Vec3::new(0.0, 40.0, 20.0),
    Vec3::new(-10.0, 20.0, 10.0),
    Vec3::new(-10.0, 20.0, -10.0),
];

/// `EnAm_Draw`'s `gDPSetEnvColor(0, 0, 0, textureBlend)`, a dynamic colour before the skeleton.
const SEG_ENV: u8 = 0x0B;
const BAKE: &str = "En_Am/skel";

/// `EnAm_Draw`'s mesh: `gArmosSkel` after `Gfx_SetupDL_25Opa`, the env colour (its alpha the
/// textures' blend) dynamic.
pub fn bakes() -> Vec<MeshBake> {
    vec![MeshBake {
        name: BAKE.into(),
        object: OBJECT.into(),
        segments: vec![(SEG_ENV, BakeSegment::DynamicColor { env: true, prim: false })],
        prelude: vec![SEG_ENV],
        body: BakeBody::Skeleton { file: OBJECT.into(), symbol: SKEL.into(), limbs: Vec::new() },
    }]
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Statue,
    Sleep,
    Lunge,
    RotateToHome,
    MoveToHome,
    RotateToInit,
    Cooldown,
    Ricochet,
    Stunned,
    RecoilFromDamage,
}

impl Action {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Action::Statue => "EnAm_Statue",
            Action::Sleep => "EnAm_Sleep",
            Action::Lunge => "EnAm_Lunge",
            Action::RotateToHome => "EnAm_RotateToHome",
            Action::MoveToHome => "EnAm_MoveToHome",
            Action::RotateToInit => "EnAm_RotateToInit",
            Action::Cooldown => "EnAm_Cooldown",
            Action::Ricochet => "EnAm_Ricochet",
            Action::Stunned => "EnAm_Stunned",
            Action::RecoilFromDamage => "EnAm_RecoilFromDamage",
        }
    }
}

struct Anims {
    /// `gArmosRicochetAnim`: its sleep's pose (the last frame) and the ricochet (frames 0 to 8).
    ricochet: Anim,
    /// `gArmosHopAnim`.
    hop: Anim,
    /// `gArmosDamagedAnim`.
    damaged: Anim,
}

impl Anims {
    fn load(play: &PlayState) -> Option<Anims> {
        let a = play.assets.clone()?;
        let get = |s: &str| a.animation(OBJECT, s).map_err(|e| log::error!("En_Am: {e:#}")).ok();
        Some(Anims { ricochet: get("gArmosRicochetAnim")?, hop: get("gArmosHopAnim")?, damaged: get("gArmosDamagedAnim")? })
    }
}

/// The DynaPoly's view of the actor (`DynaPoly_SetBgActor` reads its position, rotation, scale).
fn source(a: &Actor) -> BgActorSource {
    let r = a.shape_rot;
    BgActorSource { pos: a.world_pos, shape_rot: [r.x, r.y, r.z], scale: a.scale, shape_y_offset: a.shape_y_offset }
}

/// `ABS` of an s16 in an expression (an `int`).
fn abs_i(v: i16) -> i32 {
    (v as i32).abs()
}

/// The colliders' ids (`collider_mut`).
const COL_HURT: u8 = 0;
const COL_BLOCK: u8 = 1;
const COL_HIT: u8 = 2;

pub struct EnAm {
    /// `dyna.actor`.
    pub actor: Actor,
    /// `dyna.bgId`: the statue's `gArmosCol`; `BG_ACTOR_MAX` for the enemy (the C's
    /// `BGACTOR_NEG_ONE` from `DynaPolyActor_Init`, `bgId < 0`) or a failed registration.
    pub bg: u16,
    pub skel: SkelAnimeStd,
    skeleton: Option<Arc<Skeleton>>,
    anims: Option<Anims>,
    /// `behavior` (`ArmosBehavior`).
    pub behavior: i32,
    pub action: Action,
    /// `unk_258`: waking (Sleep), the hops left (Lunge, Cooldown, MoveToHome), the turn's state
    /// (RotateToInit), the push's phase (Statue).
    pub unk_258: i16,
    pub cooldown_timer: i16,
    /// `attackTimer`: home once it's out.
    pub attack_timer: i16,
    pub ice_timer: i16,
    /// `deathTimer`: the explosion at 0.
    pub death_timer: i16,
    /// `panicSpinRot`: the spin before the explosion.
    pub panic_spin_rot: i16,
    /// `unk_264`: the sword's quad on (in the air of a lunge).
    pub unk_264: i16,
    /// `textureBlend`: 0 the statue's textures, 255 the enemy's.
    pub texture_blend: u8,
    pub damage_reaction: u8,
    /// `shakeOrigin`: where it shakes about while waking.
    pub shake_origin: Vec3,
    pub hurt_collider: ColliderCylinder,
    pub block_collider: ColliderCylinder,
    pub hit_collider: ColliderQuad,
}

impl EnAm {
    fn anim(&self, f: impl Fn(&Anims) -> &Anim) -> Option<Anim> {
        self.anims.as_ref().map(|a| f(a).clone())
    }

    /// `Animation_Change(anim, speed, start, end (negative: the last frame plus it), mode, 0)`.
    fn change(&mut self, f: impl Fn(&Anims) -> &Anim, speed: f32, start: f32, end: impl Fn(f32) -> f32, mode: u8) {
        if let Some(a) = self.anim(f) {
            let last = a.last_frame();
            self.skel.change(a, speed, start, end(last), mode, 0.0);
        }
    }

    /// `Animation_PlayLoopSetSpeed(gArmosHopAnim, 4)`.
    fn play_hop(&mut self) {
        self.change(|a| &a.hop, 4.0, 0.0, |last| last, ANIMMODE_LOOP);
    }

    fn is_statue(&self) -> bool {
        TYPE.get(self.actor.params) == ARMOS_STATUE
    }

    /// `dyna.unk_150`, `unk_154`, `unk_158`: the statue's are its bg actor's (Player's push writes
    /// them there); the enemy's are never written (0: `DynaPolyActor_Init` and the zeroed actor).
    fn unk_150(&self, play: &PlayState) -> f32 {
        play.col.dyna.unk_150(self.bg)
    }
    fn unk_158(&self, play: &PlayState) -> i16 {
        play.col.dyna.unk_158(self.bg)
    }
    fn clear_unk_150_154(&self, play: &PlayState) {
        play.col.dyna.set_unk_154(self.bg, 0.0);
        play.col.dyna.set_unk_150(self.bg, 0.0);
    }

    /// `EnAm_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // sInitChain: naviEnemyId NAVI_ENEMY_ARMOS, gravity -4, lockOnArrowOffset 5300.
        actor.navi_enemy_id = NAVI_ENEMY_ARMOS;
        actor.gravity = -4.0;
        actor.target_arrow_offset = 5300.0;
        actor.shape_init(0.0, oot_game::shadow::ShadowDraw::Circle, 48.0);
        let skeleton = play.assets.clone().and_then(|a| a.skeleton(OBJECT, SKEL).map_err(|e| log::error!("En_Am: {e:#}")).ok());
        let anims = Anims::load(play);
        // SkelAnime_Init(gArmosSkel, gArmosRicochetAnim, .., 14): 13 limbs.
        let skel = SkelAnimeStd::init_flex(13, anims.as_ref().map(|a| a.ricochet.clone()));
        actor.scale = Vec3::splat(0.01);
        let hurt_collider = ColliderCylinder::new(&HURT_CYLINDER_INIT);
        let mut this = EnAm {
            actor,
            // DynaPolyActor_Init(&dyna, 0): bgId BGACTOR_NEG_ONE.
            bg: BG_ACTOR_MAX,
            skel,
            skeleton,
            anims,
            behavior: AM_BEHAVIOR_NONE,
            action: Action::Sleep,
            unk_258: 0,
            cooldown_timer: 0,
            attack_timer: 0,
            ice_timer: 0,
            death_timer: 0,
            panic_spin_rot: 0,
            unk_264: 0,
            texture_blend: 0,
            damage_reaction: 0,
            shake_origin: Vec3::ZERO,
            hurt_collider,
            block_collider: ColliderCylinder::default(),
            hit_collider: ColliderQuad::default(),
        };
        if this.is_statue() {
            this.actor.col_chk_info.health = 1;
            this.block_collider = ColliderCylinder::new(&HURT_CYLINDER_INIT);
            this.hurt_collider.base.oc_flags1 = OC1_ON | OC1_NO_PUSH | OC1_TYPE_1 | OC1_TYPE_2;
            this.block_collider.base.oc_flags1 = OC1_ON | OC1_NO_PUSH | OC1_TYPE_PLAYER;
            this.bg = match crate::obj_kibako2::load_collision(play, OBJECT, COLLISION) {
                Some(h) => play.col.dyna.set_bg_actor(h, source(&this.actor), 0),
                None => BG_ACTOR_MAX,
            };
            // Actor_ChangeCategory(.., ACTORCAT_BG): spawned into its new list.
            this.actor.category = ACTORCAT_BG;
            this.setup_statue();
        } else {
            this.block_collider = ColliderCylinder::new(&BLOCK_CYLINDER_INIT);
            this.hit_collider = ColliderQuad::new(&QUAD_INIT);
            this.actor.col_chk_info.health = 1;
            this.actor.col_chk_info.damage_table = Some(&S_DAMAGE_TABLE);
            this.setup_sleep();
            this.unk_258 = 0;
        }
        this.actor.col_chk_info.mass = MASS_HEAVY;
        Box::new(this)
    }

    /// `EnAm_CanMove`: whether it would stand on the floor `distance` along `yaw`
    /// (`Actor_UpdateBgCheckInfo`'s floor there, the position and `bgCheckFlags` put back), or at
    /// least on a floor no more than 20 under its home (it doesn't go down steep slopes).
    fn can_move(&mut self, play: &PlayState, distance: f32, yaw: i16) -> bool {
        let ret = actor_test_floor_in_direction(&mut self.actor, play, distance, yaw) != 0;
        ret || self.actor.floor_height >= (self.actor.home_pos.y - 20.0)
    }

    /// `EnAm_SpawnEffects`: four falling sparkles about it, 40 up (`EffectSsKiraKira_SpawnSmall`),
    /// `NA_SE_EN_AMOS_WALK` and a ring of dust (a hop's landing).
    fn spawn_effects(&self, play: &mut PlayState) {
        let velocity = Vec3::new(0.0, -1.5, 0.0);
        let accel = Vec3::new(0.0, -0.2, 0.0);
        let prim_color = [100, 100, 100, 0];
        let env_color = [40, 40, 40, 0];
        for _ in 0..4 {
            let w = self.actor.world_pos;
            let x = w.x + ((play.rand.zero_one() - 0.5) * 65.0);
            let y = (w.y + 40.0) + ((play.rand.zero_one() - 0.5) * 10.0);
            let z = w.z + ((play.rand.zero_one() - 0.5) * 65.0);
            play.with_ss(|ss| ss.kirakira_spawn_small(Vec3::new(x, y, z), velocity, accel, prim_color, env_color));
        }
        audio_play_actor_sfx2(play, NA_SE_EN_AMOS_WALK);
        let pos = self.actor.world_pos;
        actor_spawn_floor_dust_ring(play, &self.actor, pos, 4.0, 3, 8.0, 300, 15, false);
    }

    /// `EnAm_SetupSleep`: the ricochet's last frame, still.
    fn setup_sleep(&mut self) {
        if let Some(a) = self.anim(|a| &a.ricochet) {
            let last = a.last_frame();
            self.skel.change(a, 0.0, last, last, ANIMMODE_LOOP, 0.0);
        }
        self.behavior = AM_BEHAVIOR_DO_NOTHING;
        self.actor.speed_xz = 0.0;
        self.unk_258 = if self.texture_blend == 255 { 0 } else { 1 };
        self.action = Action::Sleep;
    }

    /// `EnAm_SetupStatue`: the ricochet's last frame, not targetable.
    fn setup_statue(&mut self) {
        if let Some(a) = self.anim(|a| &a.ricochet) {
            let last = a.last_frame();
            self.skel.change(a, 0.0, last, last, ANIMMODE_LOOP, 0.0);
        }
        self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        self.behavior = AM_BEHAVIOR_DO_NOTHING;
        self.actor.speed_xz = 0.0;
        self.action = Action::Statue;
    }

    /// `EnAm_SetupLunge`: three hops.
    fn setup_lunge(&mut self) {
        self.play_hop();
        self.unk_258 = 3;
        self.behavior = AM_BEHAVIOR_AGGRO;
        self.actor.speed_xz = 0.0;
        self.actor.world_rot.y = self.actor.shape_rot.y;
        self.action = Action::Lunge;
    }

    /// `EnAm_SetupCooldown`: 40 frames, then a turn to Link.
    fn setup_cooldown(&mut self) {
        self.play_hop();
        self.unk_258 = 3;
        self.cooldown_timer = 40;
        self.behavior = AM_BEHAVIOR_AGGRO;
        self.actor.speed_xz = 0.0;
        self.actor.world_rot.y = self.actor.shape_rot.y;
        self.action = Action::Cooldown;
    }

    /// `EnAm_SetupMoveToHome`.
    fn setup_move_to_home(&mut self) {
        self.play_hop();
        self.behavior = AM_BEHAVIOR_GO_HOME;
        self.unk_258 = 1;
        self.actor.speed_xz = 0.0;
        self.action = Action::MoveToHome;
    }

    /// `EnAm_SetupRotateToInit`.
    fn setup_rotate_to_init(&mut self) {
        self.play_hop();
        self.behavior = AM_BEHAVIOR_GO_HOME;
        self.unk_258 = 1;
        self.actor.speed_xz = 0.0;
        self.action = Action::RotateToInit;
    }

    /// `EnAm_SetupRotateToHome`.
    fn setup_rotate_to_home(&mut self) {
        self.play_hop();
        self.behavior = AM_BEHAVIOR_GO_HOME;
        self.actor.speed_xz = 0.0;
        self.actor.world_rot.y = self.actor.shape_rot.y;
        self.action = Action::RotateToHome;
    }

    /// `EnAm_SetupRecoilFromDamage`: the damaged animation (frames 4 to 6 before its end), facing
    /// Link, knocked back 6 if it can stand there, `NA_SE_EN_AMOS_DAMAGE`, its colour cleared, the
    /// finishing blow's freeze.
    fn setup_recoil_from_damage(&mut self, play: &mut PlayState) {
        self.change(|a| &a.damaged, 1.0, 4.0, |last| last - 6.0, ANIMMODE_ONCE);
        self.behavior = AM_BEHAVIOR_DAMAGED;
        self.actor.world_rot.y = self.actor.yaw_towards_player;
        audio_play_actor_sfx2(play, NA_SE_EN_AMOS_DAMAGE);
        if self.can_move(play, -6.0, self.actor.world_rot.y) {
            self.actor.speed_xz = -6.0;
        }
        self.actor.color_filter_timer = 0;
        enemy_start_finishing_blow(play, &self.actor);
        self.action = Action::RecoilFromDamage;
    }

    /// `EnAm_SetupRicochet`: the ricochet's frames 0 to 8, facing Link, knocked back 6 if it can
    /// stand there.
    fn setup_ricochet(&mut self, play: &PlayState) {
        self.change(|a| &a.ricochet, 1.0, 0.0, |_| 8.0, ANIMMODE_ONCE);
        self.actor.world_rot.y = self.actor.yaw_towards_player;
        if self.can_move(play, -6.0, self.actor.world_rot.y) {
            self.actor.speed_xz = -6.0;
        }
        self.unk_264 = 0;
        self.unk_258 = 0;
        self.cooldown_timer = 5;
        self.behavior = AM_BEHAVIOR_RICOCHET;
        self.action = Action::Ricochet;
    }

    /// `EnAm_Sleep`: woken by Link's touch, a hit, or already waking: the cry and a red flash
    /// from the statue's textures, shaking about where it stood as they turn (20 a frame), up at
    /// 240 (`attackTimer` 200, targetable, lunging). Otherwise its textures turn back (10 a frame);
    /// asleep it's not targetable and slides with a push (`dyna.unk_150`, which the enemy's never
    /// gets: `NA_SE_EV_ROCK_SLIDE` while it moves).
    fn sleep(&mut self, play: &mut PlayState) {
        let touched = self.hurt_collider.base.oc_flags1 & OC1_HIT != 0 && play.player.is_some() && self.hurt_collider.base.oc == play.player;
        if self.unk_258 != 0 || touched || self.hurt_collider.base.ac_flags & AC_HIT != 0 {
            self.hurt_collider.base.ac_flags &= !AC_HIT;
            if self.texture_blend == 0 {
                audio_play_actor_sfx2(play, NA_SE_EN_AMOS_WAVE);
                audio_play_actor_sfx2(play, NA_SE_EN_AMOS_VOICE);
                self.actor.set_color_filter(COLORFILTER_COLORFLAG_RED, 255, COLORFILTER_BUFFLAG_OPA, 8);
            }
            if self.texture_blend >= 240 {
                self.attack_timer = 200;
                self.texture_blend = 255;
                self.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED;
                self.actor.shape_y_offset = 0.0;
                self.setup_lunge();
            } else {
                // Shake randomly about the origin while waking up.
                let rand = (play.rand.zero_one() - 0.5) * 10.0;
                let cos = cos_s(self.actor.shape_rot.y) * rand;
                let sin = sin_s(self.actor.shape_rot.y) * rand;
                self.actor.world_pos.x = self.shake_origin.x + cos;
                self.actor.world_pos.z = self.shake_origin.z + sin;
                self.texture_blend = self.texture_blend.wrapping_add(20);
                self.unk_258 = 1;
            }
        } else if self.texture_blend > 10 {
            self.texture_blend -= 10;
        } else {
            self.texture_blend = 0;
            self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
            // dyna.bgId < 0: the enemy's (BGACTOR_NEG_ONE).
            if self.bg == BG_ACTOR_MAX {
                self.unk_264 = 0;
            }
            self.actor.speed_xz += self.unk_150(play);
            self.shake_origin = self.actor.world_pos;
            self.actor.world_rot.y = self.unk_158(play);
            self.actor.speed_xz = self.actor.speed_xz.clamp(-2.5, 2.5);
            smooth_step_to_f(&mut self.actor.speed_xz, 0.0, 1.0, 1.0, 0.0);
            if self.actor.speed_xz != 0.0 {
                audio_play_actor_sfx2(play, NA_SE_EV_ROCK_SLIDE - SFX_FLAG);
            }
            self.clear_unk_150_154(play);
        }
    }

    /// The hop's landing, on the ground: or held on frame 11 in the air. Returns whether it landed
    /// this frame (`curFrame > 11` on the ground).
    fn hop_landing(&mut self) -> bool {
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND == 0 {
            self.skel.cur_frame = 11.0;
            false
        } else {
            true
        }
    }

    /// `EnAm_RotateToHome`: hopping round towards home (up to 0x1F40 a hop), then home.
    fn rotate_to_home(&mut self, play: &mut PlayState) {
        let yaw_to_home = vec3f_yaw(self.actor.world_pos, self.actor.home_pos);
        if self.skel.cur_frame == 8.0 {
            smooth_step_to_s(&mut self.actor.world_rot.y, yaw_to_home, 1, 0x1F40, 0);
            self.actor.velocity.y = 12.0;
        } else if self.skel.cur_frame > 11.0 && self.hop_landing() {
            self.spawn_effects(play);
            if self.actor.world_rot.y == yaw_to_home {
                self.unk_258 = 0;
            }
            self.actor.velocity.y = 0.0;
            self.actor.world_pos.y = self.actor.floor_height;
        }
        self.skel.update();
        if self.unk_258 == 0 {
            self.setup_move_to_home();
        }
        self.actor.shape_rot.y = self.actor.world_rot.y;
    }

    /// `EnAm_RotateToInit`: at home, hopping round to the way it faced (and the last 8 a hop
    /// onto home), then asleep.
    fn rotate_to_init(&mut self, play: &mut PlayState) {
        if self.skel.cur_frame == 8.0 {
            if self.actor.world_pos.x == self.actor.home_pos.x && self.actor.world_pos.z == self.actor.home_pos.z {
                smooth_step_to_s(&mut self.actor.world_rot.y, self.actor.home_rot.y, 1, 0x1F40, 0);
            }
            self.unk_258 = 2;
            self.actor.velocity.y = 12.0;
        } else if self.skel.cur_frame > 11.0 && self.hop_landing() {
            self.unk_258 = 1;
            self.spawn_effects(play);
            if self.actor.home_rot.y == self.actor.world_rot.y {
                self.unk_258 = 0;
            }
            self.actor.velocity.y = 0.0;
            self.actor.world_pos.y = self.actor.floor_height;
        }
        if self.unk_258 == 2 {
            smooth_step_to_f(&mut self.actor.world_pos.x, self.actor.home_pos.x, 1.0, 8.0, 0.0);
            smooth_step_to_f(&mut self.actor.world_pos.z, self.actor.home_pos.z, 1.0, 8.0, 0.0);
        }
        self.skel.update();
        if self.unk_258 == 0 {
            self.setup_sleep();
        }
        self.actor.shape_rot.y = self.actor.world_rot.y;
    }

    /// `EnAm_MoveToHome`: hops of 6 home, turning to it (0xBB8 a landing); within 80 of it, the
    /// turn back.
    fn move_to_home(&mut self, play: &mut PlayState) {
        let yaw_to_home = vec3f_yaw(self.actor.world_pos, self.actor.home_pos);
        if self.skel.cur_frame == 8.0 {
            self.actor.velocity.y = 12.0;
            self.actor.speed_xz = 6.0;
        } else if self.skel.cur_frame > 11.0 && self.hop_landing() {
            smooth_step_to_s(&mut self.actor.world_rot.y, yaw_to_home, 1, 0xBB8, 0);
            if self.actor.bg_check_flags & BGCHECKFLAG_GROUND_TOUCH != 0 {
                self.unk_258 -= 1;
            }
            self.actor.velocity.y = 0.0;
            self.actor.speed_xz = 0.0;
            self.actor.world_pos.y = self.actor.floor_height;
            self.spawn_effects(play);
            // Actor_WorldDistXYZToPoint.
            if self.actor.world_pos.distance(self.actor.home_pos) < 80.0 {
                self.setup_rotate_to_init();
            }
        }
        // Turn away from a wall if touching one.
        if self.actor.speed_xz != 0.0 && self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 {
            self.actor.world_rot.y = self.actor.wall_yaw;
            self.actor.move_forward();
        }
        self.skel.update();
        self.actor.shape_rot.y = self.actor.world_rot.y;
    }

    /// `EnAm_RecoilFromDamage`: sliding back to a stop (no further than it can stand); the
    /// animation done, the death's hops (`deathTimer` 64).
    fn recoil_from_damage(&mut self, play: &mut PlayState) {
        if self.actor.speed_xz < 0.0 {
            self.actor.speed_xz += 0.5;
        }
        if self.actor.velocity.y <= 0.0 && !self.can_move(play, -8.0, self.actor.world_rot.y) {
            self.actor.speed_xz = 0.0;
        }
        if self.skel.update() {
            self.setup_lunge();
            self.death_timer = 64;
            self.panic_spin_rot = 0;
        }
    }

    /// `EnAm_Cooldown`: after 3 lunges, 40 frames' wait, then hops round to Link (up to 0x1F40 a
    /// hop) until it faces him within 3500, and lunges again (`NA_SE_EN_AMOS_VOICE`).
    fn cooldown(&mut self, play: &mut PlayState) {
        // yawDiff = ABS(yawDiff), back in an s16 (0x8000 stays -0x8000).
        let yaw_diff = abs_i(self.actor.yaw_towards_player.wrapping_sub(self.actor.world_rot.y)) as i16;
        if self.cooldown_timer != 0 {
            self.cooldown_timer -= 1;
        } else {
            if self.skel.cur_frame == 8.0 {
                smooth_step_to_s(&mut self.actor.world_rot.y, self.actor.yaw_towards_player, 1, 0x1F40, 0);
                self.actor.velocity.y = 12.0;
            } else if self.skel.cur_frame > 11.0 && self.hop_landing() {
                if yaw_diff < 3500 {
                    self.unk_258 = 0;
                }
                self.actor.velocity.y = 0.0;
                self.actor.world_pos.y = self.actor.floor_height;
                self.spawn_effects(play);
            }
            self.skel.update();
            if self.unk_258 == 0 {
                self.setup_lunge();
                audio_play_actor_sfx2(play, NA_SE_EN_AMOS_VOICE);
            }
            self.actor.shape_rot.y = self.actor.world_rot.y;
        }
    }

    /// `EnAm_Lunge`: hop at Link three times (12 up, 6 on if it can stand 80 ahead), its sword's
    /// quad on in the air, turning to him (0x1770) on each landing; far from home (240) or its
    /// `attackTimer` out, home. Bounced off a wall it turns away. Also the death's hops: still for
    /// its first 12 frames (`deathTimer` from 64 to 52), then spinning faster up to 8000 a frame.
    fn lunge(&mut self, play: &mut PlayState) {
        if self.death_timer < 52 {
            if self.skel.cur_frame == 8.0 {
                self.actor.velocity.y = 12.0;
                if self.can_move(play, 80.0, self.actor.world_rot.y) {
                    self.actor.speed_xz = 6.0;
                } else {
                    self.actor.speed_xz = 0.0;
                }
                self.unk_264 = 1;
                self.hit_collider.base.at_flags &= !(AT_HIT | AT_BOUNCED);
            } else if self.skel.cur_frame > 11.0 && self.hop_landing() {
                smooth_step_to_s(&mut self.actor.world_rot.y, self.actor.yaw_towards_player, 1, 0x1770, 0);
                if self.actor.bg_check_flags & BGCHECKFLAG_GROUND_TOUCH != 0 {
                    self.unk_258 -= 1;
                }
                self.actor.velocity.y = 0.0;
                self.actor.speed_xz = 0.0;
                self.unk_264 = 0;
                self.actor.world_pos.y = self.actor.floor_height;
                self.spawn_effects(play);
                // Actor_WorldDistXZToPoint.
                let d = self.actor.world_pos - self.actor.home_pos;
                let dist_xz = (d.x * d.x + d.z * d.z).sqrt();
                if (dist_xz > 240.0 || self.attack_timer == 0) && self.death_timer == 0 {
                    self.setup_rotate_to_home();
                }
            }
            // Turn and move away from a wall if contact is made with one.
            if self.actor.speed_xz != 0.0 && self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 {
                self.actor.world_rot.y = self.actor.wall_yaw.wrapping_sub(self.actor.world_rot.y).wrapping_add(self.actor.wall_yaw);
                self.actor.move_forward();
                self.actor.bg_check_flags &= !BGCHECKFLAG_WALL;
            }
            self.skel.update();
            if self.unk_258 == 0 && self.death_timer == 0 {
                self.setup_cooldown();
            }
            if self.death_timer == 0 {
                self.actor.shape_rot.y = self.actor.world_rot.y;
            } else {
                if self.panic_spin_rot < 8000 {
                    self.panic_spin_rot += 800;
                }
                self.actor.shape_rot.y = self.actor.shape_rot.y.wrapping_add(self.panic_spin_rot);
            }
        }
    }

    /// `EnAm_Statue`: a push or pull (`dyna.unk_150`) starts a 16-frame step (`unk_258` 0x8000
    /// down by 0x800: speed `sin(unk_258) × unk_150 / 2` along `unk_158`), `NA_SE_EV_ROCK_SLIDE`;
    /// it stops (and Player's push flags and speed with it) at the step's end, off the ground,
    /// at a wall (`func_800435D8`: 20 either side, 40 plus the speed ahead, 10 up) or touching
    /// an actor ahead (its hurt cylinder's OC within 0x2000 of the push). `NA_SE_EV_BLOCK_BOUND`
    /// landing. The push read, it's cleared.
    fn statue(&mut self, play: &mut PlayState) {
        let unk_150 = self.unk_150(play);
        let unk_158 = self.unk_158(play);
        let mut temp158f = unk_158 as f32;
        let mut move_dir: i16 = 0;
        if self.unk_258 == 0 {
            if unk_150 != 0.0 {
                self.unk_258 = 0x8000u16 as i16;
            }
        } else {
            self.unk_258 = self.unk_258.wrapping_sub(0x800);
            audio_play_actor_sfx2(play, NA_SE_EV_ROCK_SLIDE - SFX_FLAG);
            if unk_150 < 0.0 {
                temp158f = (unk_158 as i32 + 0x8000) as f32;
            }
            let oc_hit = self.hurt_collider.base.oc_flags1 & OC1_HIT != 0;
            if oc_hit && let Some(oc) = self.hurt_collider.base.oc.and_then(|h| play.actors.actor(h)) {
                // An s16 less a float, stored in an s16.
                move_dir = (vec3f_yaw(self.actor.world_pos, oc.world_pos) as f32 - temp158f) as i32 as i16;
            }
            let arg3 = ((sin_s(self.unk_258) * (unk_150 * 0.5)) + 40.0) as i32 as i16;
            if unk_150 == 0.0
                || self.unk_258 == 0
                || self.actor.bg_check_flags & BGCHECKFLAG_GROUND == 0
                || !crate::bg_spot15_rrbox::func_800435d8(play, &self.actor, self.bg, 0x14, arg3, 0xA)
                || (oc_hit && abs_i(move_dir) <= 0x2000)
            {
                self.unk_258 = 0;
                if let Some(h) = play.player {
                    if let Some(pi) = play.actors.get_mut(h).and_then(|p| p.as_player_mut()) {
                        pi.change_state_flags2(0, PLAYER_STATE2_0 | PLAYER_STATE2_4 | PLAYER_STATE2_6 | PLAYER_STATE2_8);
                    }
                    if let Some(p) = play.actors.actor_mut(h) {
                        p.speed_xz = 0.0;
                    }
                }
                self.clear_unk_150_154(play);
            }
            self.actor.world_rot.y = unk_158;
            self.actor.speed_xz = sin_s(self.unk_258) * (self.unk_150(play) * 0.5);
        }
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND_TOUCH != 0 {
            audio_play_actor_sfx2(play, NA_SE_EV_BLOCK_BOUND);
        }
        self.clear_unk_150_154(play);
    }

    /// `EnAm_SetupStunned`: the damaged animation set (but never played: likely copied from
    /// `EnAm_SetupRecoilFromDamage`), facing Link, knocked back 6 if it can stand there, blue 100
    /// frames (frozen for ice: `iceTimer` 48), `NA_SE_EN_GOMA_JR_FREEZE`.
    fn setup_stunned(&mut self, play: &mut PlayState) {
        self.change(|a| &a.damaged, 1.0, 0.0, |last| last, ANIMMODE_ONCE);
        self.actor.world_rot.y = self.actor.yaw_towards_player;
        if self.can_move(play, -6.0, self.actor.world_rot.y) {
            self.actor.speed_xz = -6.0;
        }
        self.actor.set_color_filter(COLORFILTER_COLORFLAG_BLUE, 120, COLORFILTER_BUFFLAG_OPA, 100);
        if self.damage_reaction == AM_DMG_REACT_ICE {
            self.ice_timer = 48;
        }
        self.behavior = AM_BEHAVIOR_STUNNED;
        audio_play_actor_sfx2(play, NA_SE_EN_GOMA_JR_FREEZE);
        self.action = Action::Stunned;
    }

    /// `EnAm_Stunned`: turning back to its way (0xFA0 a frame), sliding to a stop; once the blue's
    /// gone, lunging again, or dying with no health left.
    fn stunned(&mut self, play: &mut PlayState) {
        smooth_step_to_s(&mut self.actor.shape_rot.y, self.actor.world_rot.y, 1, 0xFA0, 0);
        if self.actor.speed_xz < 0.0 {
            self.actor.speed_xz += 0.5;
        }
        if self.actor.velocity.y <= 0.0 && !self.can_move(play, -9.0, self.actor.world_rot.y) {
            self.actor.speed_xz = 0.0;
        }
        if self.actor.color_filter_timer == 0 {
            if self.actor.col_chk_info.health != 0 {
                self.setup_lunge();
            } else {
                self.setup_recoil_from_damage(play);
            }
        }
    }

    /// `EnAm_Ricochet`: sliding back (no further than it can stand at 1.5 times its speed), the
    /// ricochet's frames played, then lunging again.
    fn ricochet(&mut self, play: &mut PlayState) {
        if self.actor.speed_xz < 0.0 {
            self.actor.speed_xz += 0.5;
        }
        if self.actor.velocity.y <= 0.0 && !self.can_move(play, self.actor.speed_xz * 1.5, self.actor.world_rot.y) {
            self.actor.speed_xz = 0.0;
        }
        if self.skel.update() {
            self.actor.speed_xz = 0.0;
            self.setup_lunge();
        }
    }

    /// `CollisionCheck_SpawnShieldParticlesMetal`: the shield's metal sparks and
    /// `NA_SE_IT_SHIELD_REFLECT_SW` (centred).
    // shared: CollisionCheck_SpawnShieldParticlesMetal (z_collision_check.c), as a call an actor
    // makes (the framework has it only inside its own hit effects).
    fn collision_check_spawn_shield_particles_metal(play: &mut PlayState, pos: Vec3) {
        play.collision_check_hit_fx(vec![HitFx::ShieldParticlesMetal(pos), HitFx::Sfx(NA_SE_IT_SHIELD_REFLECT_SW, SfxPos::Default)]);
    }

    /// `EnAm_UpdateDamage` (the enemy, not dying): its metal turned the hit away (a ricochet once
    /// it's awake); else a hit on its body once it's awake (`behavior >= AM_BEHAVIOR_5`), but fire
    /// and light magic: stunned by a nut, the boomerang, the hookshot or ice (no health with
    /// damage: ice), sparks off it when already stunned by the boomerang or the hookshot; killed by
    /// a killing hit or any while stunned; else a ricochet.
    fn update_damage(&mut self, play: &mut PlayState) {
        if self.death_timer != 0 {
            return;
        }
        if self.block_collider.base.ac_flags & AC_BOUNCED != 0 {
            self.block_collider.base.ac_flags &= !(AC_HIT | AC_BOUNCED);
            self.hurt_collider.base.ac_flags &= !AC_HIT;
            if self.behavior >= AM_BEHAVIOR_5 {
                self.setup_ricochet(play);
            }
        } else if self.hurt_collider.base.ac_flags & AC_HIT != 0 && self.behavior >= AM_BEHAVIOR_5 {
            self.hurt_collider.base.ac_flags &= !AC_HIT;
            let reaction = self.actor.col_chk_info.damage_reaction;
            if reaction != AM_DMG_REACT_MAGIC_FIRE_LIGHT {
                self.unk_264 = 0;
                self.damage_reaction = reaction;
                let elem = self.hurt_collider.info;
                self.actor.set_drop_flag(&elem, false);
                if reaction == AM_DMG_REACT_NUT || reaction == AM_DMG_REACT_STUN || reaction == AM_DMG_REACT_ICE {
                    if self.behavior != AM_BEHAVIOR_STUNNED {
                        self.setup_stunned(play);
                        if self.actor.col_chk_info.damage != 0 {
                            self.actor.col_chk_info.health = 0;
                        }
                    } else if reaction == AM_DMG_REACT_STUN {
                        let mut spark_pos = self.actor.world_pos;
                        spark_pos.y += 50.0;
                        Self::collision_check_spawn_shield_particles_metal(play, spark_pos);
                    }
                } else if reaction == AM_DMG_REACT_KILL || self.behavior == AM_BEHAVIOR_STUNNED {
                    self.actor.col_chk_info.health = 0;
                    self.setup_recoil_from_damage(play);
                } else {
                    self.setup_ricochet(play);
                }
            }
        }
    }

    fn run_action(&mut self, play: &mut PlayState) {
        match self.action {
            Action::Statue => self.statue(play),
            Action::Sleep => self.sleep(play),
            Action::Lunge => self.lunge(play),
            Action::RotateToHome => self.rotate_to_home(play),
            Action::MoveToHome => self.move_to_home(play),
            Action::RotateToInit => self.rotate_to_init(play),
            Action::Cooldown => self.cooldown(play),
            Action::Ricochet => self.ricochet(play),
            Action::Stunned => self.stunned(play),
            Action::RecoilFromDamage => self.recoil_from_damage(play),
        }
    }

    /// The death (`EnAm_Update`, `deathTimer` at 0): the landing's effects, its own bomb with no
    /// fuse (`shape.rot.z` 2: the blast's growth), `NA_SE_EN_AMOS_DEAD`, a drop from table 10, ten
    /// puffs of grey dust in a ring 7 across (by `gameplayFrames`), and it's gone.
    fn explode(&mut self, play: &mut PlayState) {
        let zero_vec = Vec3::ZERO;
        let dust_prim_color = [150, 150, 150, 255];
        let dust_env_color = [100, 100, 100, 150];
        let mut dust_pos_scale = play.gameplay_frames.wrapping_mul(10) as f32;
        self.spawn_effects(play);
        match play.actor_spawn(crate::en_bom::ACTOR_EN_BOM, self.actor.world_pos, [0, 0, 2], BOMB_BODY) {
            Ok(h) => {
                if let Some(bomb) = play.actors.downcast_mut::<crate::en_bom::EnBom>(h) {
                    bomb.timer = 0;
                }
            }
            Err(e) => log::debug!("En_Am: no bomb: {e:?}"),
        }
        audio_play_actor_sfx2(play, NA_SE_EN_AMOS_DEAD);
        let pos = self.actor.world_pos;
        crate::en_item00::item_drop_collectible_random(play, Some(&self.actor), pos, DROP_TABLE_10);
        for _ in 0..10 {
            let w = self.actor.world_pos;
            let x = (dust_pos_scale.sin() * 7.0) + w.x;
            let y = (play.rand.centered_float(10.0) * 6.0) + (w.y + 40.0);
            let z = (dust_pos_scale.cos() * 7.0) + w.z;
            play.with_ss(|s| s.func_8002836c(Vec3::new(x, y, z), zero_vec, zero_vec, dust_prim_color, dust_env_color, 200, 45, 12));
            dust_pos_scale += 60.0;
        }
        self.actor.kill();
    }

    /// `EnAm_TransformSwordHitbox` on limb 1's matrix (`EnAm_PostLimbDraw`).
    fn sword_quad(&mut self, limb: Mat4) {
        let q = SWORD_QUAD.map(|v| limb.transform_point3(v));
        self.hit_collider.set_vertices(q[0], q[1], q[2], q[3]);
    }
}

impl ActorImpl for EnAm {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnAm_Update`. @bug (game): hit by fire or light magic (`AM_DMG_REACT_MAGIC_FIRE_LIGHT`),
    /// `colChkInfo.damageReaction` keeps it until another hit, and the Armos stands still (no
    /// action, no movement) until then.
    fn update(&mut self, play: &mut PlayState) {
        if !self.is_statue() {
            self.update_damage(play);
        }
        if self.actor.col_chk_info.damage_reaction != AM_DMG_REACT_MAGIC_FIRE_LIGHT {
            if self.attack_timer != 0 {
                self.attack_timer -= 1;
            }
            self.run_action(play);
            if self.death_timer != 0 {
                self.death_timer -= 1;
                if self.death_timer == 0 {
                    self.explode(play);
                    return;
                }
                if self.death_timer % 4 == 0 {
                    self.actor.set_color_filter(COLORFILTER_COLORFLAG_RED, 255, COLORFILTER_BUFFLAG_OPA, 4);
                }
            }
            self.actor.move_forward();
            let flags = UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_2 | UPDBGCHECKINFO_FLAG_3 | UPDBGCHECKINFO_FLAG_4;
            // The C passes the actor: the statue's floor isn't its own collision.
            let own = (self.bg != BG_ACTOR_MAX).then_some(self.bg);
            self.actor.update_bg_check_info_of(&play.col, 20.0, 28.0, 80.0, flags, own);
        }
        self.hurt_collider.update(&self.actor);
        self.block_collider.update(&self.actor);
        play.collision_check_set_oc(&self.actor, COL_HURT, &mut self.hurt_collider);
        if !self.is_statue() {
            self.actor.set_focus(self.actor.scale.x * 4500.0);
            if self.actor.color_filter_timer == 0 {
                play.collision_check_set_ac(&self.actor, COL_BLOCK, &mut self.block_collider);
            }
            play.collision_check_set_ac(&self.actor, COL_HURT, &mut self.hurt_collider);
            if self.behavior >= 4 && self.unk_264 > 0 {
                if self.hit_collider.base.at_flags & AT_BOUNCED == 0 {
                    if self.hit_collider.base.at_flags & AT_HIT != 0 && play.player.is_some() && self.hit_collider.base.at == play.player {
                        audio_play_actor_sfx_at(play, play.player.unwrap(), NA_SE_PL_BODY_HIT);
                    }
                    play.collision_check_set_at(&self.actor, COL_HIT, &mut self.hit_collider);
                } else {
                    self.hit_collider.base.at_flags &= !(AT_HIT | AT_BOUNCED);
                    self.hit_collider.base.at = None;
                    self.setup_ricochet(play);
                }
            }
        } else {
            play.collision_check_set_oc(&self.actor, COL_BLOCK, &mut self.block_collider);
        }
        if self.bg != BG_ACTOR_MAX {
            play.col.dyna.set_source(self.bg, source(&self.actor));
        }
    }

    /// `EnAm_Draw`'s changes: the sword's quad on limb 1 while it's on (`EnAm_PostLimbDraw`);
    /// frozen, its colour held on and an ice chunk every 4 frames (`EffectSsEnIce`).
    fn draw_update(&mut self, play: &mut PlayState) {
        if self.unk_264 != 0
            && let Some(skeleton) = self.skeleton.clone()
        {
            let model = actor_draw_matrix(&RenderState::of(&self.actor));
            let joints = self.skel.joint_table.clone();
            let mut limb1 = None;
            draw_opa_pose(&skeleton, &joints, |_, _, _| LimbDraw::Default(Mat4::IDENTITY), |limb, m| {
                if limb == LIMB_SWORD {
                    limb1 = Some(m);
                }
            });
            if let Some(m) = limb1 {
                self.sword_quad(model * m);
            }
        }
        if self.ice_timer != 0 {
            self.actor.color_filter_timer = self.actor.color_filter_timer.wrapping_add(1);
            self.ice_timer -= 1;
            if self.ice_timer % 4 == 0 {
                let index = (self.ice_timer >> 2) as usize;
                let pos = self.actor.world_pos + ICE_POS_OFFSETS[index];
                let r = self.actor.shape_rot;
                let me = play.cur_actor.map(|h| oot_game::effect::SsActor { handle: h, world_pos: self.actor.world_pos, shape_rot: [r.x, r.y, r.z] });
                play.effect_ss_en_ice_spawn_flying_vec3f(me, pos, [150, 150, 150, 250], [235, 245, 255], 1.4);
            }
        }
    }

    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        rs.switches = vec![self.texture_blend as u32];
        rs
    }

    /// `EnAm_Draw`: `gArmosSkel` after `Gfx_SetupDL_25Opa` with env `(0, 0, 0, textureBlend)`.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let (Some(joints), Some(skeleton)) = (&rs.joints, &self.skeleton) else { return };
        let [blend] = rs.switches.as_slice() else { return };
        let bones = skeleton.pose(joints);
        let mut sv = SegmentValues::default();
        sv.env[SEG_ENV as usize] = Some([0, 0, 0, *blend as u8]);
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(BAKE)), transform: actor_draw_matrix(rs), bones, params: DrawParams { segments: Some(sv), ..Default::default() } });
    }

    /// `EnAm_Destroy`: its collision deleted (`DynaPoly_DeleteBgActor`, nothing for the enemy's
    /// -1). @bug (game): the quad isn't destroyed (destroying does nothing anyway).
    fn destroy(&mut self, play: &mut PlayState) {
        play.col.dyna.delete_bg_actor(self.bg);
    }

    fn dyna_bg_id(&self) -> Option<u16> {
        (self.bg != BG_ACTOR_MAX).then_some(self.bg)
    }

    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        match id {
            COL_HURT => Some(ColliderMut::Cylinder(&mut self.hurt_collider)),
            COL_BLOCK => Some(ColliderMut::Cylinder(&mut self.block_collider)),
            COL_HIT => Some(ColliderMut::Quad(&mut self.hit_collider)),
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
