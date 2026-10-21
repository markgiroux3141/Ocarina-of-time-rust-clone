//! `En_Rr` (`ovl_En_Rr/z_en_rr.c`): the Like Like, a tube of five pulsing rings that creeps at
//! Link, swallows him, eats his shield and tunic and spits him out (GAME-10 milestone 4).
//! `object_rr`'s `gLikeLikeDL`, no skeleton: its body is five segments (`EnRrBodySegment`), the
//! base and four rings above it, each with its height, rotation and scale. Master Quest's
//! Jabu-Jabu's Belly (`bdan_scene`, layer 0; layers 1 to 3 repeat them) places five, all params
//! 0xFFFF: room 5 two, at (1345, 387, -1448) and (1642, 480, -1686), facing 0; room 11 two, at
//! (1003, 224, -5647) facing -0x4000 and (324, 207, -5652) facing 0; room 14 one, at (1246, -1293,
//! -2470) facing 0, beside the boomerang's way. Room 4 loads `object_rr` but places none. The
//! Fire Temple's room 17 and other scenes place more.
//!
//! - **Approaching** (`EnRr_Approach`): turning to Link (`Math_SmoothStepToS`, a tenth, 0x1F4 at
//!   most), creeping at 2 when he's within 400 and it has stopped (`EnRr_Move`, slowing 0.1 a
//!   frame, `NA_SE_EN_LIKE_WALK` at each start); its count (`actionTimer`) out and Link within 160
//!   across, it **reaches** (`EnRr_Reach`, `NA_SE_EN_LIKE_UNARI`): the rings stretch up (heights 0,
//!   500, 750, 1000, 1000, narrowed to 0.8, leaning 6000) for 20 frames, the mouth opens wide
//!   (1.5 for 5 frames), gapes (2000 up for 2), closes (0.8 for 20), and it settles back
//!   (`EnRr_SetupNeutral`: 60 frames before the next reach).
//! - **The catch** (`EnRr_CollisionCheck`): Link touching either cylinder (`OC1_HIT`) while it
//!   isn't flashing, its `ocTimer` out, Link not invincible nor held: `play->grabPlayer`
//!   (`func_80852F38`, Player held: `PLAYER_STATE2_7`). Held, Link's `parent` is the Like Like; it
//!   **swallows** him (`EnRr_GrabPlayer`): lock-on off, 100 frames (`grabTimer`), pulsing faster,
//!   `NA_SE_EN_LIKE_DRINK`, `NA_SE_EN_LIKE_EAT` every 8 frames; Link pulled to its mouth (30 a
//!   frame on each axis) and down into it (`swallowOffset` to -55, 5 a frame).
//! - **The spit** (`EnRr_SetupReleasePlayer`): the count out or Link struggled free: lock-on on,
//!   110 frames before another catch; it **eats** the shield he wears (but the Mirror Shield) and
//!   the tunic (but the Kokiri Tunic) (`Inventory_DeleteEquipment`: off and out of the inventory,
//!   the Kokiri Tunic put on, `Player_SetEquipmentData`), and says so (0x305F the shield, 0x3060 the
//!   tunic, 0x3061 both; only the Deku and Hylian Shields and the Goron and Zora Tunics count,
//!   `EnRr_GetMessage`); having eaten, it **retreats** (`EnRr_Retreat`: away from Link at 2 for 100
//!   frames). Link is thrown out (`Actor_SetPlayerKnockbackLarge`: 4 along its facing, 12 up, 8
//!   damage), `NA_SE_EN_LIKE_THROW`; hit while holding him, it's hurt or dies instead.
//! - **The hits** (`EnRr_CollisionCheck`): the mouth's cylinder (`AC_HARD`) only clanks (metal
//!   sparks, `CollisionCheck_SpawnShieldParticlesMetal2`). On the body: blue blood; a normal hit
//!   (`RR_DMG_REACT_NORMAL` and the arrows' reactions) takes its damage, flashes red 40 frames
//!   (invincible as long), lets Link go if held, else **recoils** (`EnRr_Damage`: its rings
//!   swaying 5000 each way every 8 frames, `NA_SE_EN_LIKE_DAMAGE`) or **dies** with the drop the
//!   reaction sets (a light arrow a purple rupee; the unknown arrow 2 the flexible drop; the
//!   unknown arrow 1 arrows; the unknown arrow 3 (and the unknown hit) a small magic jar; the rest
//!   a random rupee from table 12). Fire burns it (red 80 frames, flames), ice freezes it (blue 80,
//!   ice; only when it isn't flashing already), light magic greys it (a red rupee on death), the
//!   boomerang and the hookshot stun it (blue 80, `NA_SE_EN_GOMA_JR_FREEZE`): **stunned**
//!   (`EnRr_Stunned`, its rings still) until the colour wears off, then it lets Link go, comes on
//!   or dies.
//! - **Death** (`EnRr_Death`, `NA_SE_EN_LIKE_DEAD`, lock-on off): 40 frames of its rings sinking
//!   and the lower ones swelling, then shrinking (0.001 a frame at most) to frame 95: a puff of
//!   smoke at frame 88 (`EffectSsDeadDb`), then at frame 95 what it ate (a Deku or Hylian Shield, a
//!   Goron or Zora Tunic: `Item_DropCollectible`) and its own drop, and it's gone.
//!
//! The whole overlay is ported. Its draw (`EnRr_Draw`) is `gLikeLikeDL` in the XLU list after
//! `Gfx_SetupDL_25Xlu`: the base under the actor's matrix scaled by the base's scale, the four rings
//! under the matrices on segment 0x0C (each 1000 plus its height above the last, turned
//! `Matrix_RotateZYX`, scaled by its own), the second texture scrolling (`Gfx_TwoTexScroll` on
//! segment 8, `scrollTimer × -6`). The mouth's place (`mouthPos`: the last ring's origin, where the
//! mouth's cylinder sits and Link is pulled to), the rings' (`effectPos`) and the fire and ice
//! effects of a burn or a freeze (`effectTimer`, holding the colour on) are `draw_update`'s. No
//! shadow (no `ActorShape_Init`). Not ported: `Rumble_Request` (no rumble).
//!
//! Params: none (placed 0xFFFF; the C never reads them).
//! Flags: none.

use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, DrawParams, MeshKey, SegmentValues};
use eng_math::{approach_f, cos_s, sin_s, smooth_step_to_s, step_to_f};
use glam::{Mat4, Vec3};
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_ENEMY, ActorImpl, ActorProfile, actor_set_player_knockback_large, audio_play_actor_sfx2};
use oot_game::actor_schema::{ActorSchema, Field, Kind, TextLine};
use oot_game::audio::sfx::{NA_SE_EN_GOMA_JR_FREEZE, NA_SE_IT_SHIELD_REFLECT_SW, SfxPos};
use oot_game::collision_check::*;
use oot_game::item::{EQUIP_TYPE_SHIELD, EQUIP_TYPE_TUNIC, EQUIP_VALUE_SHIELD_MIRROR, EQUIP_VALUE_TUNIC_KOKIRI};
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};
use oot_game::scene_table::gfx_two_tex_scroll;
use oot_game::sys_matrix::MtxF;

use crate::en_item00::{ITEM00_ARROWS_SINGLE, ITEM00_FLEXIBLE, ITEM00_MAGIC_SMALL, ITEM00_RUPEE_PURPLE, ITEM00_RUPEE_RED, ITEM00_SHIELD_DEKU, ITEM00_SHIELD_HYLIAN, ITEM00_TUNIC_GORON, ITEM00_TUNIC_ZORA};
use crate::player::{self, Player, STATE2_7};

/// `ACTOR_EN_RR` (`actor_table.h`: 0x00DD).
pub const ACTOR_EN_RR: i16 = 0x00DD;
/// `OBJECT_RR` (`object_table.h`: 0x00D4).
pub const OBJECT: &str = "object_rr";
pub const OBJECT_RR: i16 = 0x00D4;
/// `gLikeLikeDL` (`object_rr.xml`): the whole body.
pub const DL: &str = "gLikeLikeDL";

/// `ACTOR_FLAG_HOOKSHOT_PULLS_PLAYER` (`actor.h`: 1 << 10).
const ACTOR_FLAG_HOOKSHOT_PULLS_PLAYER: u32 = 1 << 10;

/// `En_Rr_Profile`: `ACTORCAT_ENEMY`, attention-enabled, hostile, always updating and drawn, the
/// hookshot pulling Link to it.
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_EN_RR,
    name: "En_Rr",
    category: ACTORCAT_ENEMY,
    flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE | ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_DRAW_CULLING_DISABLED | ACTOR_FLAG_HOOKSHOT_PULLS_PLAYER,
    object: OBJECT,
    schema: &SCHEMA,
};

/// `params`: read by nothing.
pub const PARAMS: Field = Field::whole("params", Kind::Unused, "placed as 0xFFFF; read by nothing");

/// En_Rr's configuration (docs/adr/0066-the-actor-schema.md). No objects of its own beyond its
/// profile's: what it drops on its death (`En_Item00`) needs the shields' and the tunics' objects
/// (`object_gi_shield_1`, `_2`, `object_gi_clothes`), which Jabu-Jabu's rooms load (but the
/// tunics') and the Fire Temple's room 17 doesn't (the drop is gone at once, as `En_Item00` is).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[PARAMS],
    flags: &[],
    todo: false,
    objects: &[],
    texts: &[
        TextLine::new(0x305F).cond("spitting Link out, having eaten his shield").note("your shield was eaten"),
        TextLine::new(0x3060).cond("spitting Link out, having eaten his tunic").note("your tunic was eaten"),
        TextLine::new(0x3061).cond("spitting Link out, having eaten his shield and his tunic").note("your shield and tunic were eaten"),
    ],
};

/// `RR_MESSAGE_SHIELD`, `RR_MESSAGE_TUNIC`.
pub const RR_MESSAGE_SHIELD: u8 = 1 << 0;
pub const RR_MESSAGE_TUNIC: u8 = 1 << 1;
/// `RR_MOUTH`, `RR_BASE`: the segments.
pub const RR_MOUTH: usize = 4;
pub const RR_BASE: usize = 0;

/// `EnRrReachState`.
pub const REACH_NONE: u8 = 0;
pub const REACH_EXTEND: u8 = 1;
pub const REACH_STOP: u8 = 2;
pub const REACH_OPEN: u8 = 3;
pub const REACH_GAPE: u8 = 4;
pub const REACH_CLOSE: u8 = 5;

/// `EnRrDamageReaction`.
pub const RR_DMG_REACT_NONE: u8 = 0x0;
pub const RR_DMG_REACT_STUN: u8 = 0x1;
pub const RR_DMG_REACT_FIRE: u8 = 0x2;
pub const RR_DMG_REACT_ICE: u8 = 0x3;
pub const RR_DMG_REACT_LIGHT_MAGIC: u8 = 0x4;
pub const RR_DMG_REACT_LIGHT_ARROW: u8 = 0xB;
pub const RR_DMG_REACT_SHDW_ARROW: u8 = 0xC;
pub const RR_DMG_REACT_WIND_ARROW: u8 = 0xD;
pub const RR_DMG_REACT_SPRT_ARROW: u8 = 0xE;
pub const RR_DMG_REACT_NORMAL: u8 = 0xF;

/// `EnRrDropType`.
pub const RR_DROP_RANDOM_RUPEE: u8 = 0;
pub const RR_DROP_MAGIC: u8 = 1;
pub const RR_DROP_ARROW: u8 = 2;
pub const RR_DROP_FLEXIBLE: u8 = 3;
pub const RR_DROP_RUPEE_PURPLE: u8 = 4;
pub const RR_DROP_RUPEE_RED: u8 = 5;

/// `sDropNames` (the debug print's).
const S_DROP_NAMES: [&str; 6] = ["Type 7", "Small magic jar", "Arrow", "Fairy", "20 rupees", "50 rupees"];

// The sounds (`include/tables/sfx/enemybank_table.h`).
/// `NA_SE_EN_LIKE_WALK` (0x3990).
pub const NA_SE_EN_LIKE_WALK: u16 = 0x3990;
/// `NA_SE_EN_LIKE_UNARI` (0x3991).
pub const NA_SE_EN_LIKE_UNARI: u16 = 0x3991;
/// `NA_SE_EN_LIKE_DRINK` (0x3992).
pub const NA_SE_EN_LIKE_DRINK: u16 = 0x3992;
/// `NA_SE_EN_LIKE_EAT` (0x3993).
pub const NA_SE_EN_LIKE_EAT: u16 = 0x3993;
/// `NA_SE_EN_LIKE_THROW` (0x3994).
pub const NA_SE_EN_LIKE_THROW: u16 = 0x3994;
/// `NA_SE_EN_LIKE_DAMAGE` (0x3995).
pub const NA_SE_EN_LIKE_DAMAGE: u16 = 0x3995;
/// `NA_SE_EN_LIKE_DEAD` (0x3996).
pub const NA_SE_EN_LIKE_DEAD: u16 = 0x3996;

/// `NAVI_ENEMY_LIKE_LIKE` (`actor.h`: 0x37).
pub const NAVI_ENEMY_LIKE_LIKE: u8 = 0x37;
/// `ATTENTION_RANGE_2` (`actor.h`).
const ATTENTION_RANGE_2: u8 = 2;
/// `COLLECTIBLE_DROP_RANDOM_PARAMS(COLLECTIBLE_DROP_TABLE_12, false)` (`z_en_item00.h`: the table
/// × 16).
const DROP_TABLE_12: i16 = 12 * 16;

/// `sCylinderInit1` (`ColliderCylinderInitType1`: `Collider_SetCylinderType1` gives `OC2_TYPE_1`):
/// the body, hit by Player's `0xFFCFFFFF` and the hookshot's hold (`ACELEM_HOOKABLE`), touching
/// Link; 30 by 55.
pub const CYLINDER_INIT_1: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_NONE, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_ON | OC1_TYPE_PLAYER, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0xFFCF_FFFF, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x08 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: HIT_BACKLASH_NONE, defense: 0x00 },
        at_elem_flags: ATELEM_ON | ATELEM_SFX_NORMAL,
        ac_elem_flags: ACELEM_ON | ACELEM_HOOKABLE,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 30, height: 55, y_shift: 0, pos: [0; 3] },
};

/// `sCylinderInit2` (Type1, `OC2_TYPE_1`): the mouth, hard (`AC_HARD`), touching Link without
/// pushing; 20 by 20, 10 down, at `mouthPos`.
pub const CYLINDER_INIT_2: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_NONE, ac_flags: AC_ON | AC_HARD | AC_TYPE_PLAYER, oc_flags1: OC1_ON | OC1_NO_PUSH | OC1_TYPE_PLAYER, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0xFFCF_FFFF, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x08 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: HIT_BACKLASH_NONE, defense: 0x00 },
        at_elem_flags: ATELEM_ON | ATELEM_SFX_NORMAL,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 20, height: 20, y_shift: -10, pos: [0; 3] },
};

/// `sDamageTable`.
pub static S_DAMAGE_TABLE: DamageTable = DamageTable {
    table: [
        dmg_entry(0, RR_DMG_REACT_NONE),          // Deku nut
        dmg_entry(2, RR_DMG_REACT_NORMAL),        // Deku stick
        dmg_entry(1, RR_DMG_REACT_NORMAL),        // Slingshot
        dmg_entry(2, RR_DMG_REACT_NORMAL),        // Explosive
        dmg_entry(0, RR_DMG_REACT_STUN),          // Boomerang
        dmg_entry(2, RR_DMG_REACT_NORMAL),        // Normal arrow
        dmg_entry(2, RR_DMG_REACT_NORMAL),        // Hammer swing
        dmg_entry(0, RR_DMG_REACT_STUN),          // Hookshot
        dmg_entry(1, RR_DMG_REACT_NORMAL),        // Kokiri sword
        dmg_entry(2, RR_DMG_REACT_NORMAL),        // Master sword
        dmg_entry(4, RR_DMG_REACT_NORMAL),        // Giant's Knife
        dmg_entry(4, RR_DMG_REACT_FIRE),          // Fire arrow
        dmg_entry(4, RR_DMG_REACT_ICE),           // Ice arrow
        dmg_entry(15, RR_DMG_REACT_LIGHT_ARROW),  // Light arrow
        dmg_entry(4, RR_DMG_REACT_WIND_ARROW),    // Unk arrow 1
        dmg_entry(15, RR_DMG_REACT_SHDW_ARROW),   // Unk arrow 2
        dmg_entry(15, RR_DMG_REACT_SPRT_ARROW),   // Unk arrow 3
        dmg_entry(4, RR_DMG_REACT_FIRE),          // Fire magic
        dmg_entry(3, RR_DMG_REACT_ICE),           // Ice magic
        dmg_entry(10, RR_DMG_REACT_LIGHT_MAGIC),  // Light magic
        dmg_entry(0, RR_DMG_REACT_NONE),          // Shield
        dmg_entry(0, RR_DMG_REACT_NONE),          // Mirror Ray
        dmg_entry(1, RR_DMG_REACT_NORMAL),        // Kokiri spin
        dmg_entry(4, RR_DMG_REACT_NORMAL),        // Giant spin
        dmg_entry(2, RR_DMG_REACT_NORMAL),        // Master spin
        dmg_entry(2, RR_DMG_REACT_NORMAL),        // Kokiri jump
        dmg_entry(8, RR_DMG_REACT_NORMAL),        // Giant jump
        dmg_entry(4, RR_DMG_REACT_NORMAL),        // Master jump
        dmg_entry(10, RR_DMG_REACT_SPRT_ARROW),   // Unknown 1
        dmg_entry(0, RR_DMG_REACT_NONE),          // Unblockable
        dmg_entry(0, RR_DMG_REACT_NONE),          // Hammer jump
        dmg_entry(0, RR_DMG_REACT_NONE),          // Unknown 2
    ],
};

/// The colliders' ids (`collider_mut`).
pub const COL_BODY: u8 = 0;
pub const COL_MOUTH: u8 = 1;

/// `EnRrBodySegment`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct BodySegment {
    pub height: f32,
    pub height_target: f32,
    pub scale: Vec3,
    pub scale_target: Vec3,
    pub scale_mod: Vec3,
    pub rot_target: Vec3,
    pub rot: [i16; 3],
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Approach,
    Reach,
    GrabPlayer,
    Damage,
    Death,
    Retreat,
    Stunned,
}

impl Action {
    pub fn decomp_name(self) -> &'static str {
        match self {
            Action::Approach => "EnRr_Approach",
            Action::Reach => "EnRr_Reach",
            Action::GrabPlayer => "EnRr_GrabPlayer",
            Action::Damage => "EnRr_Damage",
            Action::Death => "EnRr_Death",
            Action::Retreat => "EnRr_Retreat",
            Action::Stunned => "EnRr_Stunned",
        }
    }
}

/// `sEffectOffsets`: the burn's flames and the freeze's ice about a ring.
const S_EFFECT_OFFSETS: [Vec3; 4] = [Vec3::new(25.0, 0.0, 0.0), Vec3::new(-25.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 25.0), Vec3::new(0.0, 0.0, -25.0)];

/// The C's `(s16)` of a float (through an int, as the MIPS conversion does for these ranges).
fn s16(v: f32) -> i16 {
    v as i32 as i16
}

pub struct EnRr {
    pub actor: Actor,
    pub action: Action,
    pub collider1: ColliderCylinder,
    pub collider2: ColliderCylinder,
    pub frame_count: i16,
    pub action_timer: i16,
    pub scroll_timer: i16,
    pub grab_timer: i16,
    pub invincibility_timer: i16,
    pub effect_timer: i16,
    pub oc_timer: i16,
    /// `segMovePhase`: the pulsing's and the wobble's phase.
    pub seg_move_phase: i16,
    pub seg_phase_vel: f32,
    pub seg_phase_vel_target: f32,
    pub seg_pulse_phase_diff: f32,
    pub seg_wobble_phase_diff_x: f32,
    pub seg_wobble_x_target: f32,
    pub seg_wobble_phase_diff_z: f32,
    pub seg_wobble_z_target: f32,
    pub pulse_size: f32,
    pub pulse_size_target: f32,
    pub wobble_size: f32,
    pub wobble_size_target: f32,
    pub body_segs: [BodySegment; 5],
    pub seg_move_rate: f32,
    pub shrink_rate: f32,
    pub swallow_offset: f32,
    pub reach_state: u8,
    pub is_dead: bool,
    /// `eatenShield`: the shield's equip value (1 Deku, 2 Hylian), 0 none.
    pub eaten_shield: u8,
    /// `eatenTunic`: the tunic's equip value (2 Goron, 3 Zora), 0 none.
    pub eaten_tunic: u8,
    pub drop_type: u8,
    pub retreat: bool,
    pub stop_scroll: bool,
    pub has_player: bool,
    /// `mouthPos`: where the last draw put the mouth.
    pub mouth_pos: Vec3,
    /// `effectPos`: the actor's place, then each ring's origin, from the last draw.
    pub effect_pos: [Vec3; 5],
}

impl EnRr {
    /// `EnRr_Init`.
    pub fn init(mut actor: Actor, _play: &mut PlayState) -> Box<dyn ActorImpl> {
        // sInitChain: naviEnemyId NAVI_ENEMY_LIKE_LIKE, attentionRangeType ATTENTION_RANGE_2,
        // lockOnArrowOffset 30.
        actor.navi_enemy_id = NAVI_ENEMY_LIKE_LIKE;
        actor.target_mode = ATTENTION_RANGE_2;
        actor.target_arrow_offset = 30.0;
        actor.col_chk_info.damage_table = Some(&S_DAMAGE_TABLE);
        actor.col_chk_info.health = 4;
        actor.set_focus(30.0);
        actor.scale.y = 0.013;
        actor.scale.x = 0.014;
        actor.scale.z = 0.014;
        actor.col_chk_info.mass = MASS_IMMOVABLE;
        actor.speed_xz = 0.0;
        actor.velocity.y = 0.0;
        actor.gravity = -0.4;
        let mut this = EnRr {
            actor,
            action: Action::Approach,
            collider1: ColliderCylinder::new(&CYLINDER_INIT_1),
            collider2: ColliderCylinder::new(&CYLINDER_INIT_2),
            frame_count: 0,
            action_timer: 0,
            scroll_timer: 0,
            grab_timer: 0,
            invincibility_timer: 0,
            effect_timer: 0,
            oc_timer: 0,
            seg_move_phase: 0,
            seg_phase_vel: 0.0,
            seg_phase_vel_target: 0.0,
            seg_pulse_phase_diff: 0.0,
            seg_wobble_phase_diff_x: 0.0,
            seg_wobble_x_target: 0.0,
            seg_wobble_phase_diff_z: 0.0,
            seg_wobble_z_target: 0.0,
            pulse_size: 0.0,
            pulse_size_target: 0.0,
            wobble_size: 0.0,
            wobble_size_target: 0.0,
            body_segs: [BodySegment::default(); 5],
            seg_move_rate: 0.0,
            shrink_rate: 0.0,
            swallow_offset: 0.0,
            reach_state: REACH_NONE,
            is_dead: false,
            eaten_shield: 0,
            eaten_tunic: 0,
            drop_type: RR_DROP_RANDOM_RUPEE,
            retreat: false,
            stop_scroll: false,
            has_player: false,
            mouth_pos: Vec3::ZERO,
            effect_pos: [Vec3::ZERO; 5],
        };
        this.init_body_segments();
        Box::new(this)
    }

    /// The segments' targets every setup but the death's and the stun's resets: level, no lean,
    /// full width.
    fn reset_seg_targets(&mut self) {
        for s in self.body_segs.iter_mut() {
            s.height_target = 0.0;
            s.rot_target.x = 0.0;
            s.rot_target.z = 0.0;
            s.scale_target.x = 1.0;
            s.scale_target.z = 1.0;
        }
    }

    /// `EnRr_Move`: off at `speed`, `NA_SE_EN_LIKE_WALK`.
    fn move_(&mut self, play: &mut PlayState, speed: f32) {
        self.actor.speed_xz = speed;
        audio_play_actor_sfx2(play, NA_SE_EN_LIKE_WALK);
    }

    /// `EnRr_SetupReach`: the rings up (`segmentHeights`), narrowed and leaning; 20 frames;
    /// `NA_SE_EN_LIKE_UNARI`.
    fn setup_reach(&mut self, play: &mut PlayState) {
        const SEGMENT_HEIGHTS: [f32; 5] = [0.0, 500.0, 750.0, 1000.0, 1000.0];
        self.reach_state = REACH_EXTEND;
        self.action_timer = 20;
        self.seg_phase_vel_target = 2500.0;
        self.seg_move_rate = 0.0;
        for (s, h) in self.body_segs.iter_mut().zip(SEGMENT_HEIGHTS) {
            s.height_target = h;
            s.scale_target.x = 0.8;
            s.scale_target.z = 0.8;
            s.rot_target.x = 6000.0;
            s.rot_target.z = 0.0;
        }
        self.action = Action::Reach;
        audio_play_actor_sfx2(play, NA_SE_EN_LIKE_UNARI);
    }

    /// `EnRr_SetupNeutral`: settled; retreating 100 frames after eating, else approaching (60
    /// frames before the next reach).
    fn setup_neutral(&mut self) {
        self.reach_state = REACH_NONE;
        self.seg_move_rate = 0.0;
        self.seg_phase_vel_target = 2500.0;
        for s in self.body_segs.iter_mut() {
            s.height_target = 0.0;
            s.rot_target.x = 0.0;
            s.rot_target.z = 0.0;
            s.scale_target.x = 1.0;
            s.scale_target.z = 1.0;
        }
        if self.retreat {
            self.action_timer = 100;
            self.action = Action::Retreat;
        } else {
            self.action_timer = 60;
            self.action = Action::Approach;
        }
    }

    /// `EnRr_SetupGrabPlayer`: 100 frames, lock-on off, no catch for 8, pulsing faster;
    /// `NA_SE_EN_LIKE_DRINK`.
    fn setup_grab_player(&mut self, play: &mut PlayState) {
        self.grab_timer = 100;
        self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
        self.oc_timer = 8;
        self.has_player = true;
        self.reach_state = REACH_NONE;
        self.seg_move_rate = 0.0;
        self.swallow_offset = 0.0;
        self.actor.speed_xz = 0.0;
        self.pulse_size_target = 0.15;
        self.seg_phase_vel_target = 5000.0;
        self.wobble_size_target = 512.0;
        self.reset_seg_targets();
        self.action = Action::GrabPlayer;
        audio_play_actor_sfx2(play, NA_SE_EN_LIKE_DRINK);
    }

    /// `EnRr_GetMessage`: which of the eaten pieces it tells of (the Deku or Hylian Shield, the
    /// Goron or Zora Tunic).
    pub fn get_message(shield: u8, tunic: u8) -> u8 {
        let mut message_index = 0;
        if shield == 1 /* Deku shield */ || shield == 2 /* Hylian shield */ {
            message_index = RR_MESSAGE_SHIELD;
        }
        if tunic == 2 /* Goron tunic */ || tunic == 3 /* Zora tunic */ {
            message_index |= RR_MESSAGE_TUNIC;
        }
        message_index
    }

    /// `Inventory_DeleteEquipment(play, equipment)` (`z_inventory.c`): the save's part
    /// (`oot_game::item::inventory_delete_equipment`), then `Player_SetEquipmentData` and the pause
    /// menu's cursor on its left arrow (`PAUSE_CURSOR_PAGE_LEFT`). Returns the piece's value.
    // shared: Inventory_DeleteEquipment's play-side half (Player_SetEquipmentData, the pause
    // cursor) from an actor's update.
    fn inventory_delete_equipment(play: &mut PlayState, equipment: usize) -> u8 {
        let equip_value = oot_game::item::inventory_delete_equipment(&mut play.save, equipment);
        log::debug!("Erasing equipment item = {equipment}  zzz={equip_value}");
        if equip_value != 0 {
            let data = play.data.clone();
            if let Some(h) = play.player
                && let Some(p) = play.actors.get_mut(h).and_then(|p| p.as_player_mut())
            {
                p.set_equipment_data(&data, &play.save);
            }
            play.pause_ctx.cursor_special_pos = oot_game::kaleido::PAUSE_CURSOR_PAGE_LEFT;
        }
        equip_value as u8
    }

    /// `EnRr_SetupReleasePlayer`: see the module's docs.
    fn setup_release_player(&mut self, play: &mut PlayState) {
        self.actor.flags |= ACTOR_FLAG_ATTENTION_ENABLED;
        self.has_player = false;
        self.oc_timer = 110;
        self.seg_move_rate = 0.0;
        self.seg_phase_vel_target = 2500.0;
        self.wobble_size_target = 2048.0;
        let mut tunic = 0;
        let mut shield = 0;
        if play.save.cur_equip_value(EQUIP_TYPE_SHIELD) != EQUIP_VALUE_SHIELD_MIRROR {
            shield = Self::inventory_delete_equipment(play, EQUIP_TYPE_SHIELD);
            if shield != 0 {
                self.eaten_shield = shield;
                self.retreat = true;
            }
        }
        if play.save.cur_equip_value(EQUIP_TYPE_TUNIC) != EQUIP_VALUE_TUNIC_KOKIRI {
            tunic = Self::inventory_delete_equipment(play, EQUIP_TYPE_TUNIC);
            if tunic != 0 {
                self.eaten_tunic = tunic;
                self.retreat = true;
            }
        }
        if let Some(p) = play.player.and_then(|h| play.actors.downcast_mut::<Player>(h)) {
            p.actor.parent = None;
        }
        match Self::get_message(shield, tunic) {
            RR_MESSAGE_SHIELD => play.start_textbox(0x305F, None),
            RR_MESSAGE_TUNIC => play.start_textbox(0x3060, None),
            m if m == RR_MESSAGE_TUNIC | RR_MESSAGE_SHIELD => play.start_textbox(0x3061, None),
            _ => {}
        }
        log::debug!("../z_en_rr.c[650] : Rr_Catch_Cancel");
        actor_set_player_knockback_large(play, 4.0, self.actor.shape_rot.y, 12.0, 8);
        if self.actor.color_filter_timer == 0 {
            self.action = Action::Approach;
            audio_play_actor_sfx2(play, NA_SE_EN_LIKE_THROW);
        } else if self.actor.col_chk_info.health != 0 {
            self.setup_damage(play);
        } else {
            self.setup_death(play);
        }
    }

    /// `EnRr_SetupDamage`: 20 frames, the pulse and the wobble stopping; `NA_SE_EN_LIKE_DAMAGE`.
    fn setup_damage(&mut self, play: &mut PlayState) {
        self.reach_state = REACH_NONE;
        self.seg_move_rate = 0.0;
        self.action_timer = 20;
        self.seg_phase_vel_target = 2500.0;
        self.pulse_size_target = 0.0;
        self.wobble_size_target = 0.0;
        self.reset_seg_targets();
        self.action = Action::Damage;
        audio_play_actor_sfx2(play, NA_SE_EN_LIKE_DAMAGE);
    }

    /// `EnRr_SetupApproach`: pulsing and wobbling again.
    fn setup_approach(&mut self) {
        self.seg_move_rate = 0.0;
        self.pulse_size_target = 0.15;
        self.seg_phase_vel_target = 2500.0;
        self.wobble_size_target = 2048.0;
        self.reset_seg_targets();
        self.action = Action::Approach;
    }

    /// `EnRr_SetupDeath`: its frames counted from 0, the rings level; `NA_SE_EN_LIKE_DEAD`, lock-on
    /// off.
    fn setup_death(&mut self, play: &mut PlayState) {
        self.is_dead = true;
        self.frame_count = 0;
        self.shrink_rate = 0.0;
        self.seg_move_rate = 0.0;
        for s in self.body_segs.iter_mut() {
            s.height_target = 0.0;
            s.rot_target.x = 0.0;
            s.rot_target.z = 0.0;
        }
        self.action = Action::Death;
        audio_play_actor_sfx2(play, NA_SE_EN_LIKE_DEAD);
        self.actor.flags &= !ACTOR_FLAG_ATTENTION_ENABLED;
    }

    /// The segments' motion reset (`EnRr_SetupStunned` and `EnRr_InitBodySegments`' first part).
    fn reset_motion(&mut self) {
        self.seg_move_phase = 0;
        self.seg_phase_vel = 0.0;
        self.seg_phase_vel_target = 2500.0;
        self.seg_pulse_phase_diff = 0.0;
        self.seg_wobble_phase_diff_x = 0.0;
        self.seg_wobble_x_target = 3.0;
        self.seg_wobble_phase_diff_z = 0.0;
        self.seg_wobble_z_target = 1.0;
        self.pulse_size = 0.0;
        self.pulse_size_target = 0.15;
        self.wobble_size = 0.0;
        self.wobble_size_target = 2048.0;
        for s in self.body_segs.iter_mut() {
            s.scale_mod.y = 0.0;
            s.rot_target = Vec3::ZERO;
            s.scale = Vec3::ONE;
            s.scale_target = Vec3::ONE;
        }
    }

    /// `EnRr_SetupStunned`: its scroll stopped, its motion reset.
    fn setup_stunned(&mut self) {
        self.stop_scroll = true;
        self.reset_motion();
        self.action = Action::Stunned;
    }

    /// `CollisionCheck_SpawnShieldParticlesMetal2` (`CollisionCheck_SpawnShieldParticlesMetal`):
    /// the metal sparks and `NA_SE_IT_SHIELD_REFLECT_SW` (centred).
    // shared: CollisionCheck_SpawnShieldParticlesMetal2 (z_collision_check.c), as en_am.rs's
    // CollisionCheck_SpawnShieldParticlesMetal.
    fn collision_check_spawn_shield_particles_metal2(play: &mut PlayState, pos: Vec3) {
        play.collision_check_hit_fx(vec![HitFx::ShieldParticlesMetal(pos), HitFx::Sfx(NA_SE_IT_SHIELD_REFLECT_SW, SfxPos::Default)]);
    }

    /// `EnRr_CollisionCheck`: see the module's docs.
    fn collision_check(&mut self, play: &mut PlayState) {
        if self.collider2.base.ac_flags & AC_HIT != 0 {
            self.collider2.base.ac_flags &= !AC_HIT;
            log::debug!("Kakin ({})!!", self.frame_count);
            let h = self.collider2.info.ac_dmg_info.hit_pos;
            Self::collision_check_spawn_shield_particles_metal2(play, Vec3::new(h[0] as f32, h[1] as f32, h[2] as f32));
            return;
        }
        if self.collider1.base.ac_flags & AC_HIT != 0 {
            let mut drop_type = RR_DROP_RANDOM_RUPEE;
            self.collider1.base.ac_flags &= !AC_HIT;
            let reaction = self.actor.col_chk_info.damage_reaction;
            if reaction != 0 {
                let h = self.collider1.info.ac_dmg_info.hit_pos;
                // CollisionCheck_BlueBlood(play, NULL, &hitPos).
                play.collision_check_hit_fx(vec![HitFx::Blood(BLOOD_BLUE, Vec3::new(h[0] as f32, h[1] as f32, h[2] as f32))]);
            }
            match reaction {
                RR_DMG_REACT_LIGHT_ARROW | RR_DMG_REACT_SHDW_ARROW | RR_DMG_REACT_WIND_ARROW | RR_DMG_REACT_SPRT_ARROW | RR_DMG_REACT_NORMAL => {
                    // The fallthrough's counts: purple rupee, flexible, arrow, magic jar.
                    drop_type += match reaction {
                        RR_DMG_REACT_LIGHT_ARROW => 4,
                        RR_DMG_REACT_SHDW_ARROW => 3,
                        RR_DMG_REACT_WIND_ARROW => 2,
                        RR_DMG_REACT_SPRT_ARROW => 1,
                        _ => 0,
                    };
                    let c = &self.actor.col_chk_info;
                    log::debug!("ouch ( {} : LIFE {} : DAMAGE {} : {:x} )!!", self.frame_count, c.health, c.damage, c.damage_reaction);
                    self.stop_scroll = false;
                    self.actor.apply_damage();
                    self.invincibility_timer = 40;
                    self.actor.set_color_filter(COLORFILTER_COLORFLAG_RED, 255, COLORFILTER_BUFFLAG_XLU, self.invincibility_timer as u16);
                    if self.has_player {
                        self.setup_release_player(play);
                    } else if self.actor.col_chk_info.health != 0 {
                        self.setup_damage(play);
                    } else {
                        self.drop_type = drop_type;
                        self.setup_death(play);
                    }
                    return;
                }
                RR_DMG_REACT_FIRE => {
                    // Fire Arrow and Din's Fire.
                    self.actor.apply_damage();
                    if self.actor.col_chk_info.health == 0 {
                        self.drop_type = RR_DROP_RANDOM_RUPEE;
                    }
                    self.actor.set_color_filter(COLORFILTER_COLORFLAG_RED, 255, COLORFILTER_BUFFLAG_XLU, 80);
                    self.effect_timer = 20;
                    self.setup_stunned();
                    return;
                }
                RR_DMG_REACT_ICE => {
                    // Ice Arrow and unused ice magic (OOT_VERSION >= NTSC_1_1: only when not
                    // already tinted).
                    self.actor.apply_damage();
                    if self.actor.col_chk_info.health == 0 {
                        self.drop_type = RR_DROP_RANDOM_RUPEE;
                    }
                    if self.actor.color_filter_timer == 0 {
                        self.effect_timer = 20;
                        self.actor.set_color_filter(COLORFILTER_COLORFLAG_BLUE, 255, COLORFILTER_BUFFLAG_XLU, 80);
                    }
                    self.setup_stunned();
                    return;
                }
                RR_DMG_REACT_LIGHT_MAGIC => {
                    // Unused light magic.
                    self.actor.apply_damage();
                    if self.actor.col_chk_info.health == 0 {
                        self.drop_type = RR_DROP_RUPEE_RED;
                    }
                    self.actor.set_color_filter(COLORFILTER_COLORFLAG_GRAY, 255, COLORFILTER_BUFFLAG_XLU, 80);
                    self.setup_stunned();
                    return;
                }
                RR_DMG_REACT_STUN => {
                    // Boomerang and Hookshot.
                    audio_play_actor_sfx2(play, NA_SE_EN_GOMA_JR_FREEZE);
                    self.actor.set_color_filter(COLORFILTER_COLORFLAG_BLUE, 255, COLORFILTER_BUFFLAG_XLU, 80);
                    self.setup_stunned();
                    return;
                }
                _ => {}
            }
        }
        let Some(ph) = play.player else { return };
        let Some((invincibility_timer, state2)) = play.actors.downcast::<Player>(ph).map(|p| (p.invincibility_timer, p.state2)) else { return };
        if self.oc_timer == 0
            && self.actor.color_filter_timer == 0
            && invincibility_timer == 0
            && state2 & STATE2_7 == 0
            && (self.collider1.base.oc_flags1 & OC1_HIT != 0 || self.collider2.base.oc_flags1 & OC1_HIT != 0)
        {
            self.collider1.base.oc_flags1 &= !OC1_HIT;
            self.collider2.base.oc_flags1 &= !OC1_HIT;
            log::debug!("catch ({})!!", self.frame_count);
            if player::play_grab_player(play) {
                let me = play.cur_actor;
                if let Some(p) = play.actors.downcast_mut::<Player>(ph) {
                    p.actor.parent = me;
                }
                self.stop_scroll = false;
                self.setup_grab_player(play);
            }
        }
    }

    /// `EnRr_InitBodySegments`: the motion reset, then the pulse's scales and the wobble's leans
    /// at phase 0.
    fn init_body_segments(&mut self) {
        self.reset_motion();
        let pulse_diff = s16(self.seg_pulse_phase_diff) as u32;
        for (i, s) in self.body_segs.iter_mut().enumerate() {
            // Math_CosS(i * (u32)(s16)segPulsePhaseDiff * 0x1000).
            let m = cos_s((i as u32).wrapping_mul(pulse_diff).wrapping_mul(0x1000) as i16) * self.pulse_size;
            s.scale_mod.x = m;
            s.scale_mod.z = m;
        }
        let (diff_x, diff_z) = (s16(self.seg_wobble_phase_diff_x) as u32, s16(self.seg_wobble_phase_diff_z) as u32);
        for i in 1..5usize {
            let s = &mut self.body_segs[i];
            s.rot_target.x = cos_s((i as u32).wrapping_mul(diff_x).wrapping_mul(0x1000) as i16) * self.wobble_size;
            s.rot_target.z = sin_s((i as u32).wrapping_mul(diff_z).wrapping_mul(0x1000) as i16) * self.wobble_size;
        }
    }

    /// `EnRr_UpdateBodySegments`: alive, the pulse's scales at the phase (each ring
    /// `segPulsePhaseDiff` × 0x1000 further on), and settled (not reaching) the wobble's leans;
    /// the phase on by `segPhaseVel` unless the scroll is stopped.
    fn update_body_segments(&mut self) {
        let phase = self.seg_move_phase as i32;
        if !self.is_dead {
            let pulse_diff = s16(self.seg_pulse_phase_diff) as i32;
            for (i, s) in self.body_segs.iter_mut().enumerate() {
                let m = cos_s((phase + (i as i32) * pulse_diff * 0x1000) as i16) * self.pulse_size;
                s.scale_mod.x = m;
                s.scale_mod.z = m;
            }
            if !self.is_dead && self.reach_state == REACH_NONE {
                let (diff_x, diff_z) = (s16(self.seg_wobble_phase_diff_x) as i32, s16(self.seg_wobble_phase_diff_z) as i32);
                for i in 1..5usize {
                    let s = &mut self.body_segs[i];
                    s.rot_target.x = cos_s((phase + (i as i32) * diff_x * 0x1000) as i16) * self.wobble_size;
                    s.rot_target.z = sin_s((phase + (i as i32) * diff_z * 0x1000) as i16) * self.wobble_size;
                }
            }
        }
        if !self.stop_scroll {
            self.seg_move_phase = self.seg_move_phase.wrapping_add(s16(self.seg_phase_vel));
        }
    }

    /// `EnRr_Approach`: see the module's docs.
    fn approach(&mut self, play: &mut PlayState) {
        smooth_step_to_s(&mut self.actor.shape_rot.y, self.actor.yaw_towards_player, 0xA, 0x1F4, 0);
        self.actor.world_rot.y = self.actor.shape_rot.y;
        if self.action_timer == 0 && self.actor.xz_dist_to_player < 160.0 {
            self.setup_reach(play);
        } else if self.actor.xz_dist_to_player < 400.0 && self.actor.speed_xz == 0.0 {
            self.move_(play, 2.0);
        }
    }

    /// `EnRr_Reach`: the reach's stages, each on its count.
    fn reach(&mut self) {
        smooth_step_to_s(&mut self.actor.shape_rot.y, self.actor.yaw_towards_player, 0xA, 0x1F4, 0);
        self.actor.world_rot.y = self.actor.shape_rot.y;
        match self.reach_state {
            REACH_EXTEND => {
                if self.action_timer == 0 {
                    self.reach_state = REACH_STOP;
                }
            }
            REACH_STOP => {
                if self.action_timer == 0 {
                    self.action_timer = 5;
                    self.body_segs[RR_MOUTH].scale_target.x = 1.5;
                    self.body_segs[RR_MOUTH].scale_target.z = 1.5;
                    self.reach_state = REACH_OPEN;
                }
            }
            REACH_OPEN => {
                if self.action_timer == 0 {
                    self.action_timer = 2;
                    self.body_segs[RR_MOUTH].height_target = 2000.0;
                    self.reach_state = REACH_GAPE;
                }
            }
            REACH_GAPE => {
                if self.action_timer == 0 {
                    self.action_timer = 20;
                    self.body_segs[RR_MOUTH].scale_target.x = 0.8;
                    self.body_segs[RR_MOUTH].scale_target.z = 0.8;
                    self.reach_state = REACH_CLOSE;
                }
            }
            REACH_CLOSE => {
                if self.action_timer == 0 {
                    self.setup_neutral();
                }
            }
            _ => {}
        }
    }

    /// `EnRr_GrabPlayer`: Link pulled into its mouth until the count runs out or he struggles
    /// free; `NA_SE_EN_LIKE_EAT` every 8 frames.
    fn grab_player(&mut self, play: &mut PlayState) {
        // Rumble_Request(xyzDistToPlayerSq, 120, 2, 120): no rumble.
        if self.frame_count % 8 == 0 {
            audio_play_actor_sfx2(play, NA_SE_EN_LIKE_EAT);
        }
        self.oc_timer = 8;
        let held = play.player.and_then(|h| play.actors.downcast::<Player>(h)).is_some_and(|p| p.state2 & STATE2_7 != 0);
        if self.grab_timer == 0 || !held {
            self.setup_release_player(play);
        } else {
            let (mouth, offset) = (self.mouth_pos, self.swallow_offset);
            if let Some(p) = play.player.and_then(|h| play.actors.downcast_mut::<Player>(h)) {
                approach_f(&mut p.actor.world_pos.x, mouth.x, 1.0, 30.0);
                approach_f(&mut p.actor.world_pos.y, mouth.y + offset, 1.0, 30.0);
                approach_f(&mut p.actor.world_pos.z, mouth.z, 1.0, 30.0);
            }
            approach_f(&mut self.swallow_offset, -55.0, 1.0, 5.0);
        }
    }

    /// `EnRr_Damage`: the colour worn off, approaching again; else the rings swaying 5000 one way
    /// or the other by bit 3 of the colour's count.
    fn damage(&mut self) {
        if self.actor.color_filter_timer == 0 {
            self.setup_approach();
        } else {
            let z = if self.actor.color_filter_timer & 8 != 0 { 5000.0 } else { -5000.0 };
            for s in &mut self.body_segs[1..5] {
                s.rot_target.z = z;
            }
        }
    }

    /// `EnRr_Death`: see the module's docs.
    fn death(&mut self, play: &mut PlayState) {
        let fc = self.frame_count;
        if fc < 40 {
            for (i, s) in self.body_segs.iter_mut().enumerate() {
                approach_f(&mut s.height_target, (i as i32 + 59) as f32 - (fc as f32 * 25.0), 1.0, 50.0);
                let sq = ((4 - i as i32) * (4 - i as i32)) as f32;
                s.scale_target.x = (sq * fc as f32 * 0.003) + 1.0;
                s.scale_target.z = s.scale_target.x;
            }
        } else if fc >= 95 {
            let drop_pos = self.actor.world_pos;
            match self.eaten_shield {
                1 => {
                    crate::en_item00::item_drop_collectible(play, drop_pos, ITEM00_SHIELD_DEKU);
                }
                2 => {
                    crate::en_item00::item_drop_collectible(play, drop_pos, ITEM00_SHIELD_HYLIAN);
                }
                _ => {}
            }
            match self.eaten_tunic {
                2 => {
                    crate::en_item00::item_drop_collectible(play, drop_pos, ITEM00_TUNIC_GORON);
                }
                3 => {
                    crate::en_item00::item_drop_collectible(play, drop_pos, ITEM00_TUNIC_ZORA);
                }
                _ => {}
            }
            log::debug!("「{}」dropped!!", S_DROP_NAMES.get(self.drop_type as usize).unwrap_or(&"?"));
            match self.drop_type {
                RR_DROP_MAGIC => {
                    crate::en_item00::item_drop_collectible(play, drop_pos, ITEM00_MAGIC_SMALL);
                }
                RR_DROP_ARROW => {
                    crate::en_item00::item_drop_collectible(play, drop_pos, ITEM00_ARROWS_SINGLE);
                }
                RR_DROP_FLEXIBLE => {
                    crate::en_item00::item_drop_collectible(play, drop_pos, ITEM00_FLEXIBLE);
                }
                RR_DROP_RUPEE_PURPLE => {
                    crate::en_item00::item_drop_collectible(play, drop_pos, ITEM00_RUPEE_PURPLE);
                }
                RR_DROP_RUPEE_RED => {
                    crate::en_item00::item_drop_collectible(play, drop_pos, ITEM00_RUPEE_RED);
                }
                _ => crate::en_item00::item_drop_collectible_random(play, Some(&self.actor), drop_pos, DROP_TABLE_12),
            }
            self.actor.kill();
        } else if fc == 88 {
            let w = self.actor.world_pos;
            let pos = Vec3::new(w.x, w.y + 20.0, w.z);
            play.with_ss(|s| s.dead_db_spawn(pos, Vec3::ZERO, Vec3::ZERO, 100, 0, [255, 255, 255, 255], [255, 0, 0], 1, 9, 1));
        } else {
            approach_f(&mut self.actor.scale.x, 0.0, 1.0, self.shrink_rate);
            approach_f(&mut self.shrink_rate, 0.001, 1.0, 0.00001);
            self.actor.scale.z = self.actor.scale.x;
        }
    }

    /// `EnRr_Retreat`: away from Link (`yawTowardsPlayer + 0x8000`, 0x3E8 at most) at 2 until its
    /// count runs out, then approaching, its meal forgotten.
    fn retreat_(&mut self, play: &mut PlayState) {
        if self.action_timer == 0 {
            self.retreat = false;
            self.action = Action::Approach;
        } else {
            smooth_step_to_s(&mut self.actor.shape_rot.y, self.actor.yaw_towards_player.wrapping_add(-0x8000i16), 0xA, 0x3E8, 0);
            self.actor.world_rot.y = self.actor.shape_rot.y;
            if self.actor.speed_xz == 0.0 {
                self.move_(play, 2.0);
            }
        }
    }

    /// `EnRr_Stunned`: the colour worn off, its scroll on; Link let go if held, else approaching or
    /// dying.
    fn stunned(&mut self, play: &mut PlayState) {
        if self.actor.color_filter_timer == 0 {
            self.stop_scroll = false;
            if self.has_player {
                self.setup_release_player(play);
            } else if self.actor.col_chk_info.health != 0 {
                self.action = Action::Approach;
            } else {
                self.setup_death(play);
            }
        }
    }

    fn run_action(&mut self, play: &mut PlayState) {
        match self.action {
            Action::Approach => self.approach(play),
            Action::Reach => self.reach(),
            Action::GrabPlayer => self.grab_player(play),
            Action::Damage => self.damage(),
            Action::Death => self.death(play),
            Action::Retreat => self.retreat_(play),
            Action::Stunned => self.stunned(play),
        }
    }

    /// Each segment's draw scale: `(1 + scaleMod) × scale`.
    fn seg_scale(s: &BodySegment) -> Vec3 {
        (Vec3::ONE + s.scale_mod) * s.scale
    }

    /// `EnRr_Draw`'s matrices from `start` (the actor's): the four rings' scaled matrices (segment
    /// 0x0C) and their origins (`effectPos[1..5]`; the last is `mouthPos`).
    pub fn ring_matrices(start: MtxF, heights: [f32; 5], rots: [[i16; 3]; 5], scales: [Vec3; 5]) -> ([MtxF; 4], [Vec3; 4]) {
        let mut m = start;
        let mut out = [MtxF::IDENTITY; 4];
        let mut origins = [Vec3::ZERO; 4];
        for i in 1..5 {
            m.translate(0.0, heights[i] + 1000.0, 0.0);
            m.rotate_zyx(rots[i][0], rots[i][1], rots[i][2]);
            let mut scaled = m;
            scaled.scale(scales[i].x, scales[i].y, scales[i].z);
            out[i - 1] = scaled;
            origins[i - 1] = m.mult_vec3f(Vec3::ZERO);
        }
        (out, origins)
    }
}

/// The bake (docs/adr/0012-actor-bakes.md): `gLikeLikeDL` with segment 8 the scroll
/// (`Gfx_TwoTexScroll`, dynamic) and segment 0x0C the rings' four matrices on bones 0 to 3 (the
/// list's `gSPMatrix(0x0C000000 + k × 0x40, G_MTX_NOPUSH | G_MTX_LOAD | G_MTX_MODELVIEW)`); the base
/// drawn before any of them under the draw's own transform.
pub const BAKE: &str = "En_Rr/body";
const SEG_SCROLL: u8 = 0x08;
const SEG_MATRICES: u8 = 0x0C;

/// `EnRr_Draw`'s scroll at `scrollTimer`: `Gfx_TwoTexScroll(.., G_TX_RENDERTILE, (scrollTimer × 0)
/// & 0x7F, (scrollTimer × 0) & 0x3F, 32, 16, 1, (scrollTimer × 0) & 0x3F, (scrollTimer × -6) &
/// 0x7F, 32, 16)`: only the second tile's t moves.
pub fn scroll(scroll_timer: i16) -> Vec<(u32, u32)> {
    let t = scroll_timer as i32;
    gfx_two_tex_scroll(0, 0, 0, 32, 16, 1, 0, ((t * -6) & 0x7F) as u32, 32, 16)
}

pub fn bakes() -> Vec<MeshBake> {
    vec![MeshBake {
        name: BAKE.into(),
        object: OBJECT.into(),
        segments: vec![(SEG_SCROLL, BakeSegment::Dynamic(scroll(0))), (SEG_MATRICES, BakeSegment::Matrices(vec![0, 1, 2, 3]))],
        prelude: vec![],
        body: BakeBody::DLists(vec![(OBJECT.into(), DL.into())]),
    }]
}

/// `render_state`'s layout: values each segment's height and draw scale (x, y, z), four a segment;
/// angles each segment's rotation (x, y, z); switches `scrollTimer`.
mod rs {
    pub const PER_SEG: usize = 4;
}

impl ActorImpl for EnRr {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnRr_Update`: the counts, the segments, the hits and catches (alive, unless burning), the
    /// action, the movement, the colliders (AC and OC alive and not invincible; the body's OC not
    /// within `ocTimer`), the floor, and the segments eased to their targets (unless the scroll is
    /// stopped).
    fn update(&mut self, play: &mut PlayState) {
        self.frame_count = self.frame_count.wrapping_add(1);
        if !self.stop_scroll {
            self.scroll_timer = self.scroll_timer.wrapping_add(1);
        }
        for t in [&mut self.action_timer, &mut self.grab_timer, &mut self.oc_timer, &mut self.invincibility_timer, &mut self.effect_timer] {
            if *t != 0 {
                *t -= 1;
            }
        }
        self.actor.set_focus(30.0);
        self.update_body_segments();
        if !self.is_dead && (self.actor.color_filter_timer == 0 || self.actor.color_filter_params & 0x4000 == 0) {
            self.collision_check(play);
        }
        self.run_action(play);
        // `hasPlayer == 0x3F80`'s ASSERT (1.0f stored over it): never, `has_player` a bool.
        step_to_f(&mut self.actor.speed_xz, 0.0, 0.1);
        self.actor.move_forward();
        self.collider1.update(&self.actor);
        self.collider2.dim.pos = [s16(self.mouth_pos.x), s16(self.mouth_pos.y), s16(self.mouth_pos.z)];
        if !self.is_dead && self.invincibility_timer == 0 {
            play.collision_check_set_ac(&self.actor, COL_BODY, &mut self.collider1);
            play.collision_check_set_ac(&self.actor, COL_MOUTH, &mut self.collider2);
            if self.oc_timer == 0 {
                play.collision_check_set_oc(&self.actor, COL_BODY, &mut self.collider1);
            }
            play.collision_check_set_oc(&self.actor, COL_MOUTH, &mut self.collider2);
        } else {
            self.collider2.base.oc_flags1 &= !OC1_HIT;
            self.collider2.base.ac_flags &= !AC_HIT;
            self.collider1.base.oc_flags1 &= !OC1_HIT;
            self.collider1.base.ac_flags &= !AC_HIT;
        }
        self.actor.update_bg_check_info(&play.col, 20.0, 30.0, 20.0, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_1 | UPDBGCHECKINFO_FLAG_2);
        if !self.stop_scroll {
            approach_f(&mut self.seg_phase_vel, self.seg_phase_vel_target, 1.0, 50.0);
            approach_f(&mut self.seg_pulse_phase_diff, 4.0, 1.0, 5.0);
            approach_f(&mut self.seg_wobble_phase_diff_x, self.seg_wobble_x_target, 1.0, 0.04);
            approach_f(&mut self.seg_wobble_phase_diff_z, self.seg_wobble_z_target, 1.0, 0.01);
            approach_f(&mut self.pulse_size, self.pulse_size_target, 1.0, 0.0015);
            approach_f(&mut self.wobble_size, self.wobble_size_target, 1.0, 20.0);
            let rate = self.seg_move_rate;
            for s in self.body_segs.iter_mut() {
                smooth_step_to_s(&mut s.rot[0], s16(s.rot_target.x), 5, s16(rate * 1000.0), 0);
                smooth_step_to_s(&mut s.rot[2], s16(s.rot_target.z), 5, s16(rate * 1000.0), 0);
                approach_f(&mut s.scale.x, s.scale_target.x, 1.0, rate * 0.2);
                s.scale.z = s.scale.x;
                approach_f(&mut s.height, s.height_target, 1.0, rate * 300.0);
            }
            approach_f(&mut self.seg_move_rate, 1.0, 1.0, 0.2);
        }
    }

    /// `EnRr_Draw`'s changes: the rings' places and the mouth's (`Matrix_MultVec3f` of the origin
    /// under each ring's unscaled matrix, from `Actor_Draw`'s model matrix); burning or frozen
    /// (`effectTimer`), its colour held on and every other frame a flame (red) or a chunk of ice
    /// about a ring, from the base up.
    fn draw_update(&mut self, play: &mut PlayState) {
        let a = &self.actor;
        let r = a.shape_rot;
        let mut start = MtxF::set_translate_rotate_yxz(a.world_pos.x, a.world_pos.y + (a.shape_y_offset * a.scale.y), a.world_pos.z, [r.x, r.y, r.z]);
        start.scale(a.scale.x, a.scale.y, a.scale.z);
        let heights = self.body_segs.map(|s| s.height);
        let rots = self.body_segs.map(|s| s.rot);
        let scales = self.body_segs.map(|s| Self::seg_scale(&s));
        let (_, origins) = Self::ring_matrices(start, heights, rots, scales);
        self.effect_pos[1..5].copy_from_slice(&origins);
        self.effect_pos[0] = self.actor.world_pos;
        self.mouth_pos = origins[3];
        if self.effect_timer != 0 {
            let effect_timer = self.effect_timer - 1;
            self.actor.color_filter_timer = self.actor.color_filter_timer.wrapping_add(1);
            if effect_timer & 1 == 0 {
                let seg_index = (4 - (effect_timer >> 2)) as usize;
                let off_index = ((effect_timer >> 1) & 3) as usize;
                let base = self.effect_pos[seg_index] + S_EFFECT_OFFSETS[off_index];
                let x = base.x + play.rand.centered_float(10.0);
                let y = base.y + play.rand.centered_float(10.0);
                let z = base.z + play.rand.centered_float(10.0);
                let pos = Vec3::new(x, y, z);
                let w = self.actor.world_pos;
                let me = play.cur_actor.map(|h| oot_game::effect::SsActor { handle: h, world_pos: w, shape_rot: [r.x, r.y, r.z] });
                if self.actor.color_filter_params & 0x4000 != 0 {
                    play.effect_ss_en_fire_spawn_vec3f(me, pos, 100, 0, 0, -1);
                } else {
                    play.effect_ss_en_ice_spawn_flying_vec3f(me, pos, [150, 150, 150, 250], [235, 245, 255], 3.0);
                }
            }
        }
    }

    fn render_state(&self) -> RenderState {
        let mut r = RenderState::of(&self.actor);
        let mut values = Vec::with_capacity(5 * rs::PER_SEG);
        let mut angles = Vec::with_capacity(15);
        for s in &self.body_segs {
            let sc = Self::seg_scale(s);
            values.extend([s.height, sc.x, sc.y, sc.z]);
            angles.extend(s.rot);
        }
        r.values = values;
        r.angles = angles;
        r.switches = vec![self.scroll_timer as u16 as u32];
        r
    }

    /// `EnRr_Draw`: `gLikeLikeDL` in the XLU list (`Gfx_SetupDL_25Xlu`): the base under the
    /// actor's matrix times its scale, the rings on bones 0 to 3 (their matrices relative to that
    /// transform), the scroll at `scrollTimer`.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let (v, a) = (&rs.values, &rs.angles);
        if v.len() < 5 * rs::PER_SEG || a.len() < 15 {
            return;
        }
        let heights: [f32; 5] = std::array::from_fn(|i| v[i * rs::PER_SEG]);
        let scales: [Vec3; 5] = std::array::from_fn(|i| Vec3::new(v[i * rs::PER_SEG + 1], v[i * rs::PER_SEG + 2], v[i * rs::PER_SEG + 3]));
        let rots: [[i16; 3]; 5] = std::array::from_fn(|i| [a[i * 3], a[i * 3 + 1], a[i * 3 + 2]]);
        let base = scales[RR_BASE];
        if base.x == 0.0 || base.y == 0.0 || base.z == 0.0 {
            return;
        }
        let (rings, _) = EnRr::ring_matrices(MtxF::IDENTITY, heights, rots, scales);
        let base_inv = Mat4::from_scale(base).inverse();
        let bones = rings.iter().map(|m| base_inv * m.to_mat4()).collect();
        let mut sv = SegmentValues::default();
        let scroll_timer = rs.switches.first().copied().unwrap_or(0) as u16 as i16;
        sv.read(SEG_SCROLL, &scroll(scroll_timer));
        out.xlu.push(DrawCmd {
            mesh: MeshKey::named(keys::bake(BAKE)),
            transform: actor_draw_matrix(rs) * Mat4::from_scale(base),
            bones,
            params: DrawParams { segments: Some(sv), ..Default::default() },
        });
    }

    /// `EnRr_Destroy`: the two cylinders (nothing to free).
    fn destroy(&mut self, _play: &mut PlayState) {}

    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        match id {
            COL_BODY => Some(ColliderMut::Cylinder(&mut self.collider1)),
            COL_MOUTH => Some(ColliderMut::Cylinder(&mut self.collider2)),
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
