//! `En_Bubble` (`ovl_En_Bubble/z_en_bubble.c`): the Shabom, a soap bubble that drifts and
//! bounces about Jabu-Jabu's Belly and bursts on Link (GAME-10 milestone 4). `object_bubble`'s
//! `gBubbleDL` (a 64 by 64 `gBubbleTex` quad), drawn facing the camera.
//!
//! Master Quest's Jabu-Jabu (`bdan`) places six, all params 0xFFFF and turned (0, 0, 0): room 0's
//! three at (-219, -240, -408), (-265, -99, -359) and (-251, -190, -267), room 3's one at (26,
//! -1074, -3480) and room 12's two at (-530, -55, -5841) and (-826, -168, -5577), the same in
//! layers 0 to 3; the unused cutscene layer (4) has room 0's three elsewhere ((-129, -340, -157),
//! (93, -330, -268), (-73, -340, -398)). No other scene of this ROM places one.
//!
//! - **Drifting** (`EnBubble_Wait`, `EnBubble_Fly`): scale 1 (`EnBubble_SetDimensions`), its
//!   body 16 up (`shape.yOffset`, `(expansionHeight + 1) × 16`); set off at 3 in a random
//!   direction (`bounceDirection`, `velocityFromBounce`), sinking 0.1 faster each frame
//!   (`sinkSpeed`, down to `minVelocityY`, -20). Each frame a line 24 long from its middle along
//!   where it's going (`BgCheck_EntityLineTest1`: walls, floors and ceilings): hitting a poly, or
//!   in water going down (the surface's normal up), it **bounces** off it (`Math3D_Vec3fReflect`,
//!   normalised) at 3, or 3.6 when its bounce count (up one a bounce, back to 0 past a random 0
//!   to 9) is back to 0; the push it was given forgotten, `sinkSpeed` 0, `NA_SE_EN_AWA_BOUND`, a
//!   wobble (`graphicRotSpeed` 128, `graphicEccentricity` 0.48, both easing back to 16 and 0.08
//!   in the draw). Its velocity is the bounce's plus the push's plus `sinkSpeed`; the push eases
//!   to 0 (`Math_ApproachF`, 0.3, 0.1 at most).
//! - **Pushed** (element 1, `ACELEM_NO_DAMAGE`: the slingshot, arrows, fire and light arrows,
//!   0x2824): the attacker's velocity, normalised, times 3 added to the push (`EnBubble_Fly`),
//!   and the wobble (`func_809CC648`); it doesn't burst.
//! - **Burst** (`EnBubble_DetectPop`, 8 frames apart: `unk_208`): touching Link (`OC2_HIT_PLAYER`)
//!   it takes a quarter heart off him (`play->damagePlayer(play, -4)`, its element 0's AT
//!   damage) and pushes him back (`Actor_SetPlayerKnockbackSmallNoDamage`, 6 at 6 up); hit by
//!   anything else (element 0: everything but 0x2824, the shield and the mirror's ray) it bursts
//!   too (`func_809CBCBC`: its AT element 4 explosive, never registered). Bursting
//!   (`EnBubble_Pop`, `EnBubble_Explosion`): growing to 5 times its size and the wobble to 54 and
//!   0.2 (`Math_SmoothStepToF`, a tenth), six frames; then twenty bubbles at its middle
//!   (`EffectSsDtBubble`, white in grey, 100 to 149 big, 25 frames), a drop from table 5, lock-on
//!   off, `NA_SE_EN_AWA_BREAK` where it was, gone.
//! - Unused: `EnBubble_Disappear` (12 frames' wait, then `EnBubble_Regrow`: from nothing back to
//!   its size over 12 frames, then drifting again), and its helpers `func_809CBCEC`,
//!   `func_809CBFD4` and `func_809CC020`. Nothing sets them; they're ported whole. `unk_1F0`,
//!   `unk_1FC`, `unk_218`, `unk_21C` are set and never read.
//!
//! The whole overlay is ported. Its draw (`EnBubble_Draw`, translucent, `Gfx_SetupDL_25Xlu`):
//! `gBubbleDL` facing the camera (`Matrix_ReplaceRotation(&play->billboardMtxF)`), scaled by
//! `expansionWidth + 1` and `expansionHeight + 1` and squashed along a line turning at
//! `graphicRotSpeed` degrees a frame (`play->state.frames`) by `graphicEccentricity`. Its state
//! writes (the wobble easing back, the circle shadow's scale `(expansionWidth + 1) × 0.2`, both
//! spheres at its middle with radius `16 × (expansionWidth + 1)`: `func_809CC774`) are in
//! `draw_update`.
//!
//! Params: `params` whole, read by nothing (every placement 0xFFFF).
//! Flags: none.

use eng_collision::math3d::Sphere16;
use eng_gfx::{DrawCmd, MeshKey};
use eng_math::{approach_f, smooth_step_to_f};
use glam::{Mat4, Vec3};
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_ENEMY, ActorImpl, ActorProfile, actor_set_player_knockback_small_no_damage, audio_play_actor_sfx2};
use oot_game::actor_schema::{ActorSchema, Field, Kind};
use oot_game::collision_check::*;
use oot_game::pack::keys;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};

/// `ACTOR_EN_BUBBLE` (`actor_table.h`: 0x002D).
pub const ACTOR_EN_BUBBLE: i16 = 0x002D;
/// `OBJECT_BUBBLE` (`object_table.h`: 0x0012).
pub const OBJECT: &str = "object_bubble";
pub const OBJECT_BUBBLE: i16 = 0x0012;
/// `gBubbleDL`: the bubble.
pub const DL: &str = "gBubbleDL";

/// `En_Bubble_Profile`: `ACTORCAT_ENEMY`, attention-enabled (not hostile), `OBJECT_BUBBLE`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_BUBBLE, name: "En_Bubble", category: ACTORCAT_ENEMY, flags: ACTOR_FLAG_ATTENTION_ENABLED, object: OBJECT, schema: &SCHEMA };

/// `params`: read by nothing.
pub const PARAMS: Field = Field::whole("params", Kind::Unused, "read by nothing (every placement 0xFFFF)");

/// En_Bubble's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema { fields: &[PARAMS], flags: &[], todo: false, objects: &[], texts: &[] };

// The sounds (`include/tables/sfx/enemybank_table.h`).
/// `NA_SE_EN_AWA_BOUND` (0x3948).
pub const NA_SE_EN_AWA_BOUND: u16 = 0x3948;
/// `NA_SE_EN_AWA_BREAK` (0x3949).
pub const NA_SE_EN_AWA_BREAK: u16 = 0x3949;

/// `NAVI_ENEMY_SHABOM` (`actor.h`: 0x16).
pub const NAVI_ENEMY_SHABOM: u8 = 0x16;
/// `COLLECTIBLE_DROP_RANDOM_PARAMS(COLLECTIBLE_DROP_TABLE_5, false)` (`z_en_item00.h`: the table
/// × 16).
pub const DROP_TABLE_5: i16 = 5 * 16;

/// `sJntSphElementsInit`: element 0 (burst by everything but 0x2824, the shield and the mirror's
/// ray; its AT 4, off until it bursts) and element 1 (pushed by 0x2824: the slingshot and the
/// arrows; no damage, no hitmark, no OC); both 16 across at its middle.
pub const JNT_SPH_ELEMENTS: [ColliderJntSphElementInit; 2] = [
    ColliderJntSphElementInit {
        info: ColliderElementInit {
            elem_material: ELEM_MATERIAL_UNK0,
            at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0x0000_0000, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x04 },
            ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_D753, hit_backlash: HIT_BACKLASH_NONE, defense: 0x00 },
            at_elem_flags: ATELEM_NONE,
            ac_elem_flags: ACELEM_ON,
            oc_elem_flags: OCELEM_ON,
        },
        limb: 0,
        model_sphere: Sphere16 { center: [0, 0, 0], radius: 16 },
        scale: 100,
    },
    ColliderJntSphElementInit {
        info: ColliderElementInit {
            elem_material: ELEM_MATERIAL_UNK0,
            at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0x0000_0000, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x00 },
            ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0x0000_2824, hit_backlash: HIT_BACKLASH_NONE, defense: 0x00 },
            at_elem_flags: ATELEM_NONE,
            ac_elem_flags: ACELEM_ON | ACELEM_NO_AT_INFO | ACELEM_NO_DAMAGE | ACELEM_NO_SWORD_SFX | ACELEM_NO_HITMARK,
            oc_elem_flags: OCELEM_NONE,
        },
        limb: 0,
        model_sphere: Sphere16 { center: [0, 0, 0], radius: 16 },
        scale: 100,
    },
];

/// `sJntSphInit`: `COL_MATERIAL_HIT6`, an enemy's AT (never registered), hit by Player, pushing
/// as type 1.
pub const JNT_SPH_INIT: ColliderInit = ColliderInit { col_type: COL_MATERIAL_HIT6, at_flags: AT_ON | AT_TYPE_ENEMY, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_JNTSPH };

/// `sColChkInfoInit2`: health 1, 2 by 25, 25 down, immovable.
pub const COL_CHK_INFO_INIT2: CollisionCheckInfoInit2 = CollisionCheckInfoInit2 { health: 1, cyl_radius: 2, cyl_height: 25, cyl_y_shift: 25, mass: MASS_IMMOVABLE };

/// `DamageTable_Get(9)` (`z_collision_btltbls.c`'s `sDamageTablePresets[9]`, "Used by
/// En_Bubble"): only the AC hit is read.
pub static DAMAGE_TABLE_9: DamageTable = DamageTable {
    table: [
        dmg_entry(0, 0x0), // Deku nut
        dmg_entry(2, 0x0), // Deku stick
        dmg_entry(0, 0x0), // Slingshot
        dmg_entry(0, 0x0), // Explosive
        dmg_entry(1, 0x0), // Boomerang
        dmg_entry(0, 0x0), // Normal arrow
        dmg_entry(2, 0x0), // Hammer swing
        dmg_entry(2, 0x0), // Hookshot
        dmg_entry(1, 0x0), // Kokiri sword
        dmg_entry(2, 0x0), // Master sword
        dmg_entry(4, 0x0), // Giant's Knife
        dmg_entry(0, 0x0), // Fire arrow
        dmg_entry(4, 0x0), // Ice arrow
        dmg_entry(0, 0x0), // Light arrow
        dmg_entry(0, 0x0), // Unk arrow 1
        dmg_entry(0, 0x0), // Unk arrow 2
        dmg_entry(0, 0x0), // Unk arrow 3
        dmg_entry(4, 0x0), // Fire magic
        dmg_entry(0, 0x0), // Ice magic
        dmg_entry(0, 0x0), // Light magic
        dmg_entry(0, 0x0), // Shield
        dmg_entry(0, 0x0), // Mirror Ray
        dmg_entry(1, 0x0), // Kokiri spin
        dmg_entry(2, 0x0), // Giant spin
        dmg_entry(4, 0x0), // Master spin
        dmg_entry(2, 0x0), // Kokiri jump
        dmg_entry(4, 0x0), // Giant jump
        dmg_entry(8, 0x0), // Master jump
        dmg_entry(0, 0x0), // Unknown 1
        dmg_entry(0, 0x0), // Unblockable
        dmg_entry(0, 0x0), // Hammer jump
        dmg_entry(0, 0x0), // Unknown 2
    ],
};

/// `sEffectAccel`, `sEffectPrimColor`, `sEffectEnvColor`: the burst's bubbles.
const EFFECT_ACCEL: Vec3 = Vec3::new(0.0, -0.5, 0.0);
const EFFECT_PRIM_COLOR: [u8; 4] = [255, 255, 255, 255];
const EFFECT_ENV_COLOR: [u8; 4] = [150, 150, 150, 0];

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Wait,
    Pop,
    /// Unused.
    Disappear,
    /// Unused.
    Regrow,
}

impl Action {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Action::Wait => "EnBubble_Wait",
            Action::Pop => "EnBubble_Pop",
            Action::Disappear => "EnBubble_Disappear",
            Action::Regrow => "EnBubble_Regrow",
        }
    }
}

pub struct EnBubble {
    pub actor: Actor,
    pub action: Action,
    pub collider: ColliderJntSph,
    /// Set but never used: a tenth of the pusher's velocity.
    pub unk_1f0: Vec3,
    /// Randomly generated, set but never used.
    pub unk_1fc: Vec3,
    /// 8 frames after a hit before the next is looked at.
    pub unk_208: i16,
    pub explosion_countdown: i16,
    pub graphic_rot_speed: f32,
    pub graphic_eccentricity: f32,
    /// Set to 1, never used.
    pub unk_218: f32,
    pub unk_21c: f32,
    pub expansion_width: f32,
    pub expansion_height: f32,
    pub bounce_count: u8,
    pub bounce_direction: Vec3,
    pub velocity_from_bounce: Vec3,
    pub normalized_attacker_velocity: Vec3,
    pub velocity_from_attack: Vec3,
    pub sink_speed: f32,
}

/// `Math3D_Vec3fReflect` (`sys_math3d.c`): `vec` reflected off the plane of `normal`.
// shared: Math3D_Vec3fReflect (and Math3D_Cos), not yet in eng_collision::math3d.
pub fn math3d_vec3f_reflect(vec: Vec3, normal: Vec3) -> Vec3 {
    let neg = vec * -1.0;
    // Math3D_Cos(&negVec, normal): Math3D_CosOut, 0 when the magnitudes' product IS_ZERO.
    let mag_product = neg.length() * normal.length();
    let vec_dot_norm = if mag_product.abs() < 0.008 { 0.0 } else { ((neg.x * normal.x) + (neg.y * normal.y) + (neg.z * normal.z)) / mag_product };
    let n = normal * vec_dot_norm;
    Vec3::new(((n.x + vec.x) + (n.x + vec.x)) + neg.x, ((n.y + vec.y) + (n.y + vec.y)) + neg.y, ((n.z + vec.z) + (n.z + vec.z)) + neg.z)
}

/// `EnBubble_Vec3fNormalize`: `vec` made 1 long, or 0 if it's 0.
pub fn vec3f_normalize(vec: Vec3) -> Vec3 {
    let norm = ((vec.x * vec.x) + (vec.y * vec.y) + (vec.z * vec.z)).sqrt();
    if norm != 0.0 { Vec3::new(vec.x / norm, vec.y / norm, vec.z / norm) } else { Vec3::ZERO }
}

/// `EnBubble_Vec3fNormalizedReflect`.
pub fn vec3f_normalized_reflect(vec1: Vec3, vec2: Vec3) -> Vec3 {
    vec3f_normalize(math3d_vec3f_reflect(vec1, vec2))
}

/// `DECR(x)`: down one unless 0; the new value.
fn decr(x: &mut i16) -> i16 {
    if *x != 0 {
        *x -= 1;
    }
    *x
}

impl EnBubble {
    /// `EnBubble_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        actor.shape_init(16.0, oot_game::shadow::ShadowDraw::Circle, 0.2);
        actor.col_chk_info.set_info2(Some(&DAMAGE_TABLE_9), &COL_CHK_INFO_INIT2);
        actor.navi_enemy_id = NAVI_ENEMY_SHABOM;
        let mut this = EnBubble {
            actor,
            action: Action::Wait,
            collider: ColliderJntSph::new(&JNT_SPH_INIT, &JNT_SPH_ELEMENTS),
            unk_1f0: Vec3::ZERO,
            unk_1fc: Vec3::ZERO,
            unk_208: 0,
            explosion_countdown: 0,
            graphic_rot_speed: 0.0,
            graphic_eccentricity: 0.0,
            unk_218: 0.0,
            unk_21c: 0.0,
            expansion_width: 0.0,
            expansion_height: 0.0,
            bounce_count: 0,
            bounce_direction: Vec3::ZERO,
            velocity_from_bounce: Vec3::ZERO,
            normalized_attacker_velocity: Vec3::ZERO,
            velocity_from_attack: Vec3::ZERO,
            sink_speed: 0.0,
        };
        let x = play.rand.zero_one();
        let y = play.rand.zero_one();
        let z = play.rand.zero_one();
        this.bounce_direction = vec3f_normalize(Vec3::new(x, y, z));
        this.velocity_from_bounce = this.bounce_direction * 3.0;
        this.set_dimensions(0.0, play);
        this.action = Action::Wait;
        Box::new(this)
    }

    /// `EnBubble_SetDimensions`: lock-on on, scale 1, 16 up, the wobble at rest, `dim` bigger.
    pub fn set_dimensions(&mut self, dim: f32, play: &mut PlayState) {
        self.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED;
        // Actor_SetScale(&this->actor, 1.0f).
        self.actor.scale = Vec3::splat(1.0);
        self.actor.shape_y_offset = 16.0;
        self.graphic_rot_speed = 16.0;
        self.graphic_eccentricity = 0.08;
        self.expansion_width = dim;
        self.expansion_height = dim;
        let a = play.rand.zero_one();
        let b = play.rand.zero_one();
        let c = play.rand.zero_one();
        self.unk_218 = 1.0;
        self.unk_21c = 1.0;
        let d = (a * a) + (b * b) + (c * c);
        self.unk_1fc = Vec3::new(a / d, b / d, c / d);
    }

    /// `func_809CBCBC`: its AT element made a 4-damage explosion (never registered), stopped;
    /// six frames to burst.
    fn func_809cbcbc(&mut self) -> i16 {
        let elem = &mut self.collider.elements[0].info;
        elem.at_dmg_info.dmg_flags = DMG_EXPLOSIVE;
        elem.at_dmg_info.hit_special_effect = HIT_SPECIAL_EFFECT_NONE;
        elem.at_dmg_info.damage = 4;
        elem.at_elem_flags = ATELEM_ON;
        self.actor.velocity.y = 0.0;
        6
    }

    /// `func_809CBCEC` (only called in an unused action): shrunk to nothing, 12 frames.
    fn func_809cbcec(&mut self, play: &mut PlayState) -> i16 {
        self.set_dimensions(-1.0, play);
        12
    }

    /// `EnBubble_DamagePlayer`: its element 0's AT damage off Link (`play->damagePlayer`), and a
    /// push back from it (6 at 6 up).
    fn damage_player(&mut self, play: &mut PlayState) {
        let damage = -(self.collider.elements[0].info.at_dmg_info.damage as i32);
        crate::player::play_damage_player(play, damage);
        actor_set_player_knockback_small_no_damage(play, 6.0, self.actor.yaw_towards_player, 6.0);
    }

    /// `EnBubble_Explosion`: growing and wobbling faster; at the countdown's end twenty bubbles,
    /// a drop, lock-on off, and a random 90 to 149 (else -1).
    fn explosion(&mut self, play: &mut PlayState) -> i32 {
        smooth_step_to_f(&mut self.expansion_width, 4.0, 0.1, 1000.0, 0.0);
        smooth_step_to_f(&mut self.expansion_height, 4.0, 0.1, 1000.0, 0.0);
        smooth_step_to_f(&mut self.graphic_rot_speed, 54.0, 0.1, 1000.0, 0.0);
        smooth_step_to_f(&mut self.graphic_eccentricity, 0.2, 0.1, 1000.0, 0.0);
        self.actor.shape_y_offset = (self.expansion_height + 1.0) * 16.0;
        if decr(&mut self.explosion_countdown) != 0 {
            return -1;
        }
        let w = self.actor.world_pos;
        let effect_pos = Vec3::new(w.x, w.y + self.actor.shape_y_offset, w.z);
        for _ in 0..20 {
            let vx = (play.rand.zero_one() - 0.5) * 7.0;
            let vy = play.rand.zero_one() * 7.0;
            let vz = (play.rand.zero_one() - 0.5) * 7.0;
            let scale = play.rand.s16_offset(100, 50);
            let velocity = Vec3::new(vx, vy, vz);
            play.with_ss(|s| s.dt_bubble_spawn_custom_color(effect_pos, velocity, EFFECT_ACCEL, EFFECT_PRIM_COLOR, EFFECT_ENV_COLOR, scale, 0x19, 0));
        }
        crate::en_item00::item_drop_collectible_random(play, None, w, DROP_TABLE_5);
        self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        play.rand.s16_offset(90, 60) as i32
    }

    /// `func_809CBFD4` (only called in an unused action): the countdown, then
    /// `func_809CBCEC`'s 12.
    fn func_809cbfd4(&mut self, play: &mut PlayState) -> i32 {
        if decr(&mut self.explosion_countdown) != 0 {
            // A u32's -1, read back as an s32.
            return -1;
        }
        self.func_809cbcec(play) as i32
    }

    /// `func_809CC020` (only called in an unused action): growing a twelfth a frame; true at the
    /// countdown's end.
    fn func_809cc020(&mut self) -> bool {
        self.expansion_width += 1.0 / 12.0;
        self.expansion_height += 1.0 / 12.0;
        decr(&mut self.explosion_countdown) == 0
    }

    /// The bounce `EnBubble_Fly` makes off a poly or the water's surface (`normal`): `dir`
    /// reflected, the count up (back to 0 past a random 0 to 9), 3.6 when it's 0 else 3, the
    /// push forgotten, the sinking stopped, `NA_SE_EN_AWA_BOUND`, the wobble.
    fn bounce(&mut self, dir: &mut Vec3, normal: Vec3, play: &mut PlayState) {
        *dir = vec3f_normalized_reflect(*dir, normal);
        self.bounce_direction = *dir;
        self.bounce_count = self.bounce_count.wrapping_add(1);
        if self.bounce_count as i32 > (play.rand.zero_one() * 10.0) as i16 as i32 {
            self.bounce_count = 0;
        }
        let bounce_speed = if self.bounce_count == 0 { 3.6000001 } else { 3.0 };
        self.velocity_from_attack = Vec3::ZERO;
        self.velocity_from_bounce = self.bounce_direction * bounce_speed;
        self.sink_speed = 0.0;
        audio_play_actor_sfx2(play, NA_SE_EN_AWA_BOUND);
        self.graphic_rot_speed = 128.0;
        self.graphic_eccentricity = 0.48;
    }

    /// `EnBubble_Fly`: see the module's docs.
    fn fly(&mut self, play: &mut PlayState) {
        if self.collider.elements[1].info.ac_elem_flags & ACELEM_HIT != 0 {
            let attacker_velocity = self.collider.base.ac.and_then(|h| play.actors.actor(h)).map(|a| a.velocity).unwrap_or(Vec3::ZERO);
            self.normalized_attacker_velocity = vec3f_normalize(attacker_velocity);
            self.velocity_from_attack += self.normalized_attacker_velocity * 3.0;
        }
        self.sink_speed -= 0.1;
        if self.sink_speed < self.actor.min_velocity_y {
            self.sink_speed = self.actor.min_velocity_y;
        }
        let b = self.velocity_from_bounce;
        let a = self.velocity_from_attack;
        let mut sp54 = vec3f_normalize(Vec3::new(b.x + a.x, b.y + a.y + self.sink_speed, b.z + a.z));
        let w = self.actor.world_pos;
        let sp78 = Vec3::new(w.x, w.y + self.actor.shape_y_offset, w.z);
        let sp6c = Vec3::new(sp78.x + (sp54.x * 24.0), sp78.y + (sp54.y * 24.0), sp78.z + (sp54.z * 24.0));
        // BgCheck_EntityLineTest1(.., chkWall true, chkFloor true, chkCeil true, chkOneFace false).
        if let Some((_, poly)) = play.col.entity_line_test(sp78, sp6c, true, true, true, false) {
            let sp60 = play.col.poly_normal(poly);
            self.bounce(&mut sp54, sp60, play);
        } else if self.actor.bg_check_flags & BGCHECKFLAG_WATER != 0 && sp54.y < 0.0 {
            self.bounce(&mut sp54, Vec3::Y, play);
        }
        let b = self.velocity_from_bounce;
        let a = self.velocity_from_attack;
        self.actor.velocity = Vec3::new(b.x + a.x, b.y + a.y + self.sink_speed, b.z + a.z);
        approach_f(&mut self.velocity_from_attack.x, 0.0, 0.3, 0.1);
        approach_f(&mut self.velocity_from_attack.y, 0.0, 0.3, 0.1);
        approach_f(&mut self.velocity_from_attack.z, 0.0, 0.3, 0.1);
    }

    /// `func_809CC648`: a hit (`AC_HIT`, forgotten): on element 1 (a push) a tenth of the
    /// pusher's velocity kept and the wobble, no burst; else 8 frames' wait and a burst.
    fn func_809cc648(&mut self, play: &PlayState) -> bool {
        if self.collider.base.ac_flags & AC_HIT == 0 {
            return false;
        }
        self.collider.base.ac_flags &= !AC_HIT;
        if self.collider.elements[1].info.ac_elem_flags & ACELEM_HIT != 0 {
            let v = self.collider.base.ac.and_then(|h| play.actors.actor(h)).map(|a| a.velocity).unwrap_or(Vec3::ZERO);
            self.unk_1f0 = Vec3::new(v.x / 10.0, v.y / 10.0, v.z / 10.0);
            self.graphic_rot_speed = 128.0;
            self.graphic_eccentricity = 0.48;
            return false;
        }
        self.unk_208 = 8;
        true
    }

    /// `EnBubble_DetectPop`: not while bursting nor within 8 frames of a hit; touching Link it
    /// hurts him and bursts; else `func_809CC648`.
    fn detect_pop(&mut self, play: &mut PlayState) -> bool {
        if decr(&mut self.unk_208) != 0 || self.action == Action::Pop {
            return false;
        }
        if self.collider.base.oc_flags2 & OC2_HIT_PLAYER != 0 {
            self.collider.base.oc_flags2 &= !OC2_HIT_PLAYER;
            self.damage_player(play);
            self.unk_208 = 8;
            return true;
        }
        self.func_809cc648(play)
    }

    /// `func_809CC774`: both spheres at its middle (the draw's matrix on the model sphere's
    /// centre, 0: the actor's translation, `shape.yOffset` up at scale 1), 16 ×
    /// `(expansionWidth + 1)` across.
    pub fn func_809cc774(&mut self) {
        // Matrix_MultVec3f(&src, &dest): the model sphere's centre is (0, 0, 0), so the draw's
        // matrix gives its translation, Actor_Draw's (the position, shape.yOffset × scale.y up).
        let w = self.actor.world_pos;
        let dest = Vec3::new(w.x, w.y + (self.actor.shape_y_offset * self.actor.scale.y), w.z);
        let dim = &mut self.collider.elements[0].dim;
        dim.world_sphere.center = [dest.x as i16, dest.y as i16, dest.z as i16];
        dim.world_sphere.radius = (dim.model_sphere.radius as f32 * (1.0 + self.expansion_width)) as i16;
        let dim = *dim;
        self.collider.elements[1].dim = dim;
    }

    /// `EnBubble_Wait`: bursting (`EnBubble_Pop`, six frames) or drifting, hittable and pushing.
    fn wait(&mut self, play: &mut PlayState) {
        if self.detect_pop(play) {
            self.explosion_countdown = self.func_809cbcbc();
            self.action = Action::Pop;
        } else {
            self.fly(play);
            self.actor.shape_y_offset = (self.expansion_height + 1.0) * 16.0;
            play.collision_check_set_ac(&self.actor, 0, &mut self.collider);
            play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
        }
    }

    /// `EnBubble_Pop`: the burst; done, `NA_SE_EN_AWA_BREAK` where it was (60 frames) and gone.
    fn pop(&mut self, play: &mut PlayState) {
        if self.explosion(play) >= 0 {
            play.sfx_source_play_sfx_at_fixed_world_pos(self.actor.world_pos, 60, NA_SE_EN_AWA_BREAK);
            self.actor.kill();
        }
    }

    /// `EnBubble_Disappear` (unused): 12 frames, then regrowing (the circle shadow back).
    fn disappear(&mut self, play: &mut PlayState) {
        let temp_v0 = self.func_809cbfd4(play);
        if temp_v0 >= 0 {
            self.actor.shape_shadow_draw = oot_game::shadow::ShadowDraw::Circle;
            self.explosion_countdown = temp_v0 as i16;
            self.action = Action::Regrow;
        }
    }

    /// `EnBubble_Regrow` (unused): growing back, then drifting; hittable and pushing.
    fn regrow(&mut self, play: &mut PlayState) {
        if self.func_809cc020() {
            self.action = Action::Wait;
        }
        play.collision_check_set_ac(&self.actor, 0, &mut self.collider);
        play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
    }
}

impl ActorImpl for EnBubble {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnBubble_Update`: moved by last frame's velocity (`Actor_UpdatePos`), the bg check (16
    /// high, 16 across, no ceiling; walls, ceiling, floor and water), the action, the focus at
    /// its middle.
    fn update(&mut self, play: &mut PlayState) {
        self.actor.update_pos();
        self.actor.update_bg_check_info(&play.col, 16.0, 16.0, 0.0, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_1 | UPDBGCHECKINFO_FLAG_2);
        match self.action {
            Action::Wait => self.wait(play),
            Action::Pop => self.pop(play),
            Action::Disappear => self.disappear(play),
            Action::Regrow => self.regrow(play),
        }
        self.actor.set_focus(self.actor.shape_y_offset);
    }

    /// `EnBubble_Destroy`: the spheres (nothing to free).
    fn destroy(&mut self, _play: &mut PlayState) {}

    /// `EnBubble_Draw`'s writes (unless it's `EnBubble_Disappear`): the wobble easing back to 16
    /// and 0.08 (`Math_SmoothStepToF`, a fifth), the shadow's scale, the spheres
    /// (`func_809CC774`).
    fn draw_update(&mut self, _play: &mut PlayState) {
        if self.action == Action::Disappear {
            return;
        }
        smooth_step_to_f(&mut self.graphic_rot_speed, 16.0, 0.2, 1000.0, 0.0);
        smooth_step_to_f(&mut self.graphic_eccentricity, 0.08, 0.2, 1000.0, 0.0);
        self.actor.shape_shadow_scale = (self.expansion_width + 1.0) * 0.2;
        self.func_809cc774();
    }

    fn render_state(&self) -> RenderState {
        let mut r = RenderState::of(&self.actor);
        r.switches = vec![(self.action != Action::Disappear) as u32];
        r.values = vec![self.expansion_width, self.expansion_height, self.graphic_rot_speed, self.graphic_eccentricity];
        r
    }

    /// `EnBubble_Draw`: `gBubbleDL` (translucent) facing the camera, scaled, squashed along a
    /// line turning `graphicRotSpeed` degrees a frame.
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let ([drawn], [w, h, rot_speed, ecc]) = (rs.switches.as_slice(), rs.values.as_slice()) else { return };
        if *drawn == 0 {
            return;
        }
        // DEG_TO_RAD((f32)play->state.frames) * graphicRotSpeed.
        let angle = ((play.state_frames as f32) * (std::f32::consts::PI / 180.0)) * rot_speed;
        let angle_neg = ((-(play.state_frames as f32)) * (std::f32::consts::PI / 180.0)) * rot_speed;
        let m = oot_game::draw::replace_rotation(actor_draw_matrix(rs), play.billboard_mtx())
            * Mat4::from_scale(Vec3::new(w + 1.0, h + 1.0, 1.0))
            * Mat4::from_rotation_z(angle)
            * Mat4::from_scale(Vec3::new(ecc + 1.0, 1.0, 1.0))
            * Mat4::from_rotation_z(angle_neg);
        out.xlu.push(DrawCmd::new(MeshKey::named(keys::mesh(OBJECT, DL)), m));
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
