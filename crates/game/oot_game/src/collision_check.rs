//! The collision check (`z_collision_check.c`): actors' colliders, and the per-frame checks
//! between them.
//!
//! - **AT / AC:** attacks. An AT collider (a sword's quad) that overlaps an AC collider (a
//!   bush's cylinder) of a matching type, with a damage flag in common, marks both as hit
//!   (`CollisionCheck_AT`). The AC side then takes damage from its damage table
//!   (`CollisionCheck_Damage`).
//! - **OC:** bodies. Overlapping OC colliders push their actors apart by mass
//!   (`CollisionCheck_OC`), through `colChkInfo.displacement`, which `Actor_UpdatePos` adds.
//!
//! ## The frame
//!
//! Actors register their colliders during their update (and Player its sword from its draw)
//! with `CollisionCheck_SetAT` / `SetAC` / `SetOC`, which reset the collider's hit flags for
//! the new frame. At the top of the next `Play_Update`, before `Actor_UpdateAll`, the checks
//! run over everything registered, then the lists are cleared. So an actor reads, in its
//! update, what its colliders hit since its last update.
//!
//! ## Ownership (docs/adr/0011-collision-check.md)
//!
//! In the game the context holds pointers into the actors' structs. Here an actor keeps its
//! colliders, and the context holds a `ColliderRef` (the actor's handle and which of its
//! colliders, `ActorImpl::collider_mut`). The checks take every registered collider out of its
//! actor, run over the owned set exactly as the C runs over its pointers, then put them back:
//! the collider state at check time is whatever the actor left, as with the pointers.
//!
//! The cross-collider pointers (`atHit`, `acHitElem`, ...) are references by handle. The hit
//! element's data an actor reads later (`acHitElem->atDmgInfo.dmgFlags`) is copied into the
//! reference when the hit is recorded (`HitElem`); in the game it is read through the pointer,
//! and it only changes when the other actor re-initialises the collider.
//!
//! `CollisionCheck_LineOCCheck` (a segment against the OC colliders, for the talk camera) runs
//! over a copy of the registered OC colliders' shapes (`OcLines`), taken when it's needed.
//!
//! `CollisionCheck_HitEffects` makes its sounds (the sword's strikes, the shield's bounce, the
//! wood's); `check` hands them back in the C's order, for the play state to play. Not ported:
//! its hit marks, blood and sparks (effects), the SAC list mode (unused), OC lines (unused), and
//! colliders without an actor.

use crate::audio::sfx::SfxPos;
use eng_collision::math3d::{self, Cylinder16, Sphere16, TriNorm};
use eng_math::is_zero;
use glam::{Mat4, Vec3};

use crate::actor::Actor;
use crate::actor_ctx::{ActorContext, ActorHandle};

pub const COLLISION_CHECK_AT_MAX: usize = 50;
pub const COLLISION_CHECK_AC_MAX: usize = 60;
pub const COLLISION_CHECK_OC_MAX: usize = 50;

// `ColliderShape`.
pub const COLSHAPE_JNTSPH: u8 = 0;
pub const COLSHAPE_CYLINDER: u8 = 1;
pub const COLSHAPE_TRIS: u8 = 2;
pub const COLSHAPE_QUAD: u8 = 3;
pub const COLSHAPE_MAX: u8 = 4;

// `ColliderMaterial`.
pub const COL_MATERIAL_HIT0: u8 = 0;
pub const COL_MATERIAL_HIT1: u8 = 1;
pub const COL_MATERIAL_HIT2: u8 = 2;
pub const COL_MATERIAL_HIT3: u8 = 3;
pub const COL_MATERIAL_HIT4: u8 = 4;
pub const COL_MATERIAL_HIT5: u8 = 5;
pub const COL_MATERIAL_HIT6: u8 = 6;
pub const COL_MATERIAL_HIT7: u8 = 7;
pub const COL_MATERIAL_HIT8: u8 = 8;
pub const COL_MATERIAL_METAL: u8 = 9;
pub const COL_MATERIAL_NONE: u8 = 10;
pub const COL_MATERIAL_WOOD: u8 = 11;
pub const COL_MATERIAL_HARD: u8 = 12;
pub const COL_MATERIAL_TREE: u8 = 13;

// `ElementMaterial`.
pub const ELEM_MATERIAL_UNK0: u8 = 0;
pub const ELEM_MATERIAL_UNK1: u8 = 1;
pub const ELEM_MATERIAL_UNK2: u8 = 2;
pub const ELEM_MATERIAL_UNK3: u8 = 3;
pub const ELEM_MATERIAL_UNK4: u8 = 4;
pub const ELEM_MATERIAL_UNK5: u8 = 5;
pub const ELEM_MATERIAL_UNK6: u8 = 6;

pub const AT_NONE: u8 = 0;
pub const AT_ON: u8 = 1 << 0;
pub const AT_HIT: u8 = 1 << 1;
pub const AT_BOUNCED: u8 = 1 << 2;
pub const AT_TYPE_PLAYER: u8 = 1 << 3;
pub const AT_TYPE_ENEMY: u8 = 1 << 4;
pub const AT_TYPE_OTHER: u8 = 1 << 5;
pub const AT_SELF: u8 = 1 << 6;
pub const AT_TYPE_ALL: u8 = AT_TYPE_PLAYER | AT_TYPE_ENEMY | AT_TYPE_OTHER;

pub const AC_NONE: u8 = 0;
pub const AC_ON: u8 = 1 << 0;
pub const AC_HIT: u8 = 1 << 1;
pub const AC_HARD: u8 = 1 << 2;
pub const AC_TYPE_PLAYER: u8 = AT_TYPE_PLAYER;
pub const AC_TYPE_ENEMY: u8 = AT_TYPE_ENEMY;
pub const AC_TYPE_OTHER: u8 = AT_TYPE_OTHER;
pub const AC_NO_DAMAGE: u8 = 1 << 6;
pub const AC_BOUNCED: u8 = 1 << 7;
pub const AC_TYPE_ALL: u8 = AC_TYPE_PLAYER | AC_TYPE_ENEMY | AC_TYPE_OTHER;

pub const OC1_NONE: u8 = 0;
pub const OC1_ON: u8 = 1 << 0;
pub const OC1_HIT: u8 = 1 << 1;
pub const OC1_NO_PUSH: u8 = 1 << 2;
pub const OC1_TYPE_PLAYER: u8 = 1 << 3;
pub const OC1_TYPE_1: u8 = 1 << 4;
pub const OC1_TYPE_2: u8 = 1 << 5;
pub const OC1_TYPE_ALL: u8 = OC1_TYPE_PLAYER | OC1_TYPE_1 | OC1_TYPE_2;

pub const OC2_NONE: u8 = 0;
pub const OC2_HIT_PLAYER: u8 = 1 << 0;
pub const OC2_UNK1: u8 = 1 << 1;
pub const OC2_UNK2: u8 = 1 << 2;
pub const OC2_TYPE_PLAYER: u8 = OC1_TYPE_PLAYER;
pub const OC2_TYPE_1: u8 = OC1_TYPE_1;
pub const OC2_TYPE_2: u8 = OC1_TYPE_2;
pub const OC2_FIRST_ONLY: u8 = 1 << 6;

pub const ATELEM_NONE: u8 = 0;
pub const ATELEM_ON: u8 = 1 << 0;
pub const ATELEM_HIT: u8 = 1 << 1;
pub const ATELEM_NEAREST: u8 = 1 << 2;
pub const ATELEM_SFX_NORMAL: u8 = 0 << 3;
pub const ATELEM_SFX_HARD: u8 = 1 << 3;
pub const ATELEM_SFX_WOOD: u8 = 2 << 3;
pub const ATELEM_SFX_NONE: u8 = 3 << 3;
pub const ATELEM_AT_HITMARK: u8 = 1 << 5;
pub const ATELEM_DREW_HITMARK: u8 = 1 << 6;
pub const ATELEM_UNK7: u8 = 1 << 7;

pub const ACELEM_NONE: u8 = 0;
pub const ACELEM_ON: u8 = 1 << 0;
pub const ACELEM_HIT: u8 = 1 << 1;
pub const ACELEM_HOOKABLE: u8 = 1 << 2;
pub const ACELEM_NO_AT_INFO: u8 = 1 << 3;
pub const ACELEM_NO_DAMAGE: u8 = 1 << 4;
pub const ACELEM_NO_SWORD_SFX: u8 = 1 << 5;
pub const ACELEM_NO_HITMARK: u8 = 1 << 6;
pub const ACELEM_DRAW_HITMARK: u8 = 1 << 7;

pub const OCELEM_NONE: u8 = 0;
pub const OCELEM_ON: u8 = 1 << 0;
pub const OCELEM_HIT: u8 = 1 << 1;
pub const OCELEM_UNK3: u8 = 1 << 3;

// Damage flags (`DMG_*`).
pub const DMG_DEKU_NUT: u32 = 1 << 0;
pub const DMG_DEKU_STICK: u32 = 1 << 1;
pub const DMG_SLINGSHOT: u32 = 1 << 2;
pub const DMG_EXPLOSIVE: u32 = 1 << 3;
pub const DMG_BOOMERANG: u32 = 1 << 4;
pub const DMG_ARROW_NORMAL: u32 = 1 << 5;
pub const DMG_HAMMER_SWING: u32 = 1 << 6;
pub const DMG_HOOKSHOT: u32 = 1 << 7;
pub const DMG_SLASH_KOKIRI: u32 = 1 << 8;
pub const DMG_SLASH_MASTER: u32 = 1 << 9;
pub const DMG_SLASH_GIANT: u32 = 1 << 10;
pub const DMG_ARROW_FIRE: u32 = 1 << 11;
pub const DMG_ARROW_ICE: u32 = 1 << 12;
pub const DMG_ARROW_LIGHT: u32 = 1 << 13;
pub const DMG_ARROW_UNK1: u32 = 1 << 14;
pub const DMG_ARROW_UNK2: u32 = 1 << 15;
pub const DMG_ARROW_UNK3: u32 = 1 << 16;
pub const DMG_MAGIC_FIRE: u32 = 1 << 17;
pub const DMG_MAGIC_ICE: u32 = 1 << 18;
pub const DMG_MAGIC_LIGHT: u32 = 1 << 19;
pub const DMG_SHIELD: u32 = 1 << 20;
pub const DMG_MIR_RAY: u32 = 1 << 21;
pub const DMG_SPIN_KOKIRI: u32 = 1 << 22;
pub const DMG_SPIN_GIANT: u32 = 1 << 23;
pub const DMG_SPIN_MASTER: u32 = 1 << 24;
pub const DMG_JUMP_KOKIRI: u32 = 1 << 25;
pub const DMG_JUMP_GIANT: u32 = 1 << 26;
pub const DMG_JUMP_MASTER: u32 = 1 << 27;
pub const DMG_UNKNOWN_1: u32 = 1 << 28;
pub const DMG_UNBLOCKABLE: u32 = 1 << 29;
pub const DMG_HAMMER_JUMP: u32 = 1 << 30;
pub const DMG_UNKNOWN_2: u32 = 1 << 31;
pub const DMG_SLASH: u32 = DMG_SLASH_KOKIRI | DMG_SLASH_MASTER | DMG_SLASH_GIANT;
pub const DMG_SPIN_ATTACK: u32 = DMG_SPIN_KOKIRI | DMG_SPIN_MASTER | DMG_SPIN_GIANT;
pub const DMG_JUMP_SLASH: u32 = DMG_JUMP_KOKIRI | DMG_JUMP_MASTER | DMG_JUMP_GIANT;
pub const DMG_SWORD: u32 = DMG_SLASH | DMG_SPIN_ATTACK | DMG_JUMP_SLASH;
pub const DMG_HAMMER: u32 = DMG_HAMMER_SWING | DMG_HAMMER_JUMP;
pub const DMG_FIRE: u32 = DMG_ARROW_FIRE | DMG_MAGIC_FIRE;
pub const DMG_ARROW: u32 = DMG_ARROW_NORMAL | DMG_ARROW_FIRE | DMG_ARROW_ICE | DMG_ARROW_LIGHT | DMG_ARROW_UNK1 | DMG_ARROW_UNK2 | DMG_ARROW_UNK3;
pub const DMG_RANGED: u32 = DMG_ARROW | DMG_HOOKSHOT | DMG_SLINGSHOT;
pub const DMG_DEFAULT: u32 = !(DMG_SHIELD | DMG_MIR_RAY);

/// `CollisionCheck_GetSwordDamage`: a sword's (or the hammer's, or a Deku Stick's) damage by
/// its flags: the Kokiri Sword's slash and spin 1; its jump slash, the Master Sword's slash and
/// spin, the hammer's swing and a stick 2; the hammer's jump, the Master Sword's jump slash and
/// the Giant's Knife's slash and spin 4; the Giant's Knife's jump slash 8; else 0. (The debug
/// build also writes it to `KREG(7)`, a debug register: not kept.)
pub fn collision_check_get_sword_damage(dmg_flags: u32) -> u8 {
    if dmg_flags & (DMG_SPIN_KOKIRI | DMG_SLASH_KOKIRI) != 0 {
        1
    } else if dmg_flags & (DMG_JUMP_KOKIRI | DMG_SPIN_MASTER | DMG_SLASH_MASTER | DMG_HAMMER_SWING | DMG_DEKU_STICK) != 0 {
        2
    } else if dmg_flags & (DMG_HAMMER_JUMP | DMG_JUMP_MASTER | DMG_SPIN_GIANT | DMG_SLASH_GIANT) != 0 {
        4
    } else if dmg_flags & DMG_JUMP_GIANT != 0 {
        8
    } else {
        0
    }
}

/// `DMG_ENTRY(damage, reaction)`: a damage table entry, the damage in the low nibble and the
/// reaction (the actor's own `damageReaction` codes) above it.
pub const fn dmg_entry(damage: u8, reaction: u8) -> u8 {
    damage | (reaction << 4)
}

// `HitSpecialEffect` (`collision_check.h`): what an AT element's hit does besides the damage,
// as `colChkInfo.acHitSpecialEffect` tells the actor it hit.
pub const HIT_SPECIAL_EFFECT_NONE: u8 = 0;
pub const HIT_SPECIAL_EFFECT_FIRE: u8 = 1;
pub const HIT_SPECIAL_EFFECT_ICE: u8 = 2;
pub const HIT_SPECIAL_EFFECT_ELECTRIC: u8 = 3;
pub const HIT_SPECIAL_EFFECT_KNOCKBACK: u8 = 4;
/// `HIT_SPECIAL_EFFECT_7` to `_9`: the same effect as `HIT_SPECIAL_EFFECT_NONE`.
pub const HIT_SPECIAL_EFFECT_7: u8 = 7;
pub const HIT_SPECIAL_EFFECT_8: u8 = 8;
pub const HIT_SPECIAL_EFFECT_9: u8 = 9;

// `HitBacklash`: what an AC element does to an attacker that hits it (`colChkInfo.atHitBacklash`).
pub const HIT_BACKLASH_NONE: u8 = 0;
pub const HIT_BACKLASH_ELECTRIC: u8 = 1;

/// `MASS_IMMOVABLE`, `MASS_HEAVY` (`actor.h`).
pub const MASS_IMMOVABLE: u8 = 0xFF;
pub const MASS_HEAVY: u8 = 0xFE;

/// `DamageTable`: per damage-flag bit, `DMG_ENTRY(damage, reaction)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DamageTable {
    pub table: [u8; 32],
}

impl DamageTable {
    /// `CollisionCheck_ApplyDamage`'s lookup: the entry at the index of the attack's highest
    /// damage flag (the loop shifts the flags down until they're 1, the lower bits shifted out).
    ///
    /// @bug (game): with no flag the loop runs to 32 and the C reads the byte after the table,
    /// whatever follows it in the overlay's data. Here that reads 0 (no damage, no reaction).
    pub fn lookup(&self, dmg_flags: u32) -> u8 {
        let mut flags = dmg_flags;
        let mut i = 0;
        while i < 32 {
            if flags == 1 {
                break;
            }
            i += 1;
            flags >>= 1;
        }
        self.table.get(i).copied().unwrap_or(0)
    }
}

/// `CollisionCheckInfo`: an actor's collision properties and this frame's results.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CollisionCheckInfo {
    pub damage_table: Option<&'static DamageTable>,
    /// Added to the position by `Actor_UpdatePos` (the OC push-out).
    pub displacement: Vec3,
    pub cyl_radius: i16,
    pub cyl_height: i16,
    pub cyl_y_shift: i16,
    pub mass: u8,
    pub health: u8,
    /// `damage`: what this frame's hits take off `health` (`Actor_ApplyDamage`).
    pub damage: u8,
    /// `damageReaction`: the damage table entry's reaction to this frame's hit.
    pub damage_reaction: u8,
    /// `atHitBacklash` (`HIT_BACKLASH_*`): from the AC element this actor's attack hit.
    pub at_hit_backlash: u8,
    /// `acHitSpecialEffect` (`HIT_SPECIAL_EFFECT_*`): from the AT element that hit this actor.
    pub ac_hit_special_effect: u8,
}

impl CollisionCheckInfo {
    /// `CollisionCheck_InitInfo` (from `Actor_Init`).
    pub fn new() -> CollisionCheckInfo {
        CollisionCheckInfo {
            damage_table: None,
            displacement: Vec3::ZERO,
            cyl_radius: 10,
            cyl_height: 10,
            cyl_y_shift: 0,
            mass: 50,
            health: 8,
            damage: 0,
            damage_reaction: 0,
            at_hit_backlash: 0,
            ac_hit_special_effect: 0,
        }
    }

    /// `CollisionCheck_ResetDamage`: after each actor's turn in `Actor_UpdateAll`.
    pub fn reset_damage(&mut self) {
        self.damage = 0;
        self.damage_reaction = 0;
        self.at_hit_backlash = 0;
        self.ac_hit_special_effect = 0;
        self.displacement = Vec3::ZERO;
    }

    /// `CollisionCheck_SetInfo`.
    pub fn set_info(&mut self, table: Option<&'static DamageTable>, init: &CollisionCheckInfoInit) {
        self.health = init.health;
        self.damage_table = table;
        self.cyl_radius = init.cyl_radius;
        self.cyl_height = init.cyl_height;
        self.mass = init.mass;
    }

    /// `CollisionCheck_SetInfo2`.
    pub fn set_info2(&mut self, table: Option<&'static DamageTable>, init: &CollisionCheckInfoInit2) {
        self.health = init.health;
        self.damage_table = table;
        self.cyl_radius = init.cyl_radius;
        self.cyl_height = init.cyl_height;
        self.cyl_y_shift = init.cyl_y_shift;
        self.mass = init.mass;
    }
}

impl Default for CollisionCheckInfo {
    fn default() -> CollisionCheckInfo {
        CollisionCheckInfo::new()
    }
}

/// `CollisionCheckInfoInit`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollisionCheckInfoInit {
    pub health: u8,
    pub cyl_radius: i16,
    pub cyl_height: i16,
    pub mass: u8,
}

/// `CollisionCheckInfoInit2`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollisionCheckInfoInit2 {
    pub health: u8,
    pub cyl_radius: i16,
    pub cyl_height: i16,
    pub cyl_y_shift: i16,
    pub mass: u8,
}

/// A collider: its actor, and which of the actor's colliders (`ActorImpl::collider_mut`'s id).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ColliderRef {
    pub actor: ActorHandle,
    pub id: u8,
}

/// An element of a collider (a sphere of a `ColliderJntSph`, a triangle of a `ColliderTris`,
/// or the single element of a cylinder or quad).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ElemRef {
    pub col: ColliderRef,
    pub elem: u16,
}

/// `atHitElem` / `acHitElem`: the element that hit, and its data as it was then.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HitElem {
    pub elem: ElemRef,
    pub at_dmg_info: ColliderElementDamageInfoAT,
    pub ac_dmg_info: ColliderElementDamageInfoAC,
    pub elem_material: u8,
}

/// `ColliderElementDamageInfoAT`: what an element does as an attack.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ColliderElementDamageInfoAT {
    pub dmg_flags: u32,
    /// `hitSpecialEffect` (`HIT_SPECIAL_EFFECT_*`): what the hit does besides the damage.
    pub hit_special_effect: u8,
    pub damage: u8,
}

/// `ColliderElementDamageInfoAC`: what an element takes as an attack target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColliderElementDamageInfoAC {
    pub dmg_flags: u32,
    /// `hitBacklash` (`HIT_BACKLASH_*`): what hitting it does to the attacker.
    pub hit_backlash: u8,
    pub defense: u8,
    pub hit_pos: [i16; 3],
}

impl Default for ColliderElementDamageInfoAC {
    /// `Collider_InitElementDamageInfoAC`.
    fn default() -> ColliderElementDamageInfoAC {
        ColliderElementDamageInfoAC { dmg_flags: 0xFFCF_FFFF, hit_backlash: 0, defense: 0, hit_pos: [0; 3] }
    }
}

/// `ColliderElementDamageInfoACInit`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColliderElementDamageInfoACInit {
    pub dmg_flags: u32,
    pub hit_backlash: u8,
    pub defense: u8,
}

/// `Collider`: the part common to every shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColliderBase {
    /// `actor`: set when the collider is first registered (`CollisionCheck_Set*`).
    pub actor: Option<ActorHandle>,
    /// `at`, `ac`, `oc`: the actor on the other side of this frame's hit.
    pub at: Option<ActorHandle>,
    pub ac: Option<ActorHandle>,
    pub oc: Option<ActorHandle>,
    pub at_flags: u8,
    pub ac_flags: u8,
    pub oc_flags1: u8,
    pub oc_flags2: u8,
    pub col_type: u8,
    pub shape: u8,
}

impl Default for ColliderBase {
    /// `Collider_InitBase`.
    fn default() -> ColliderBase {
        ColliderBase { actor: None, at: None, ac: None, oc: None, at_flags: AT_NONE, ac_flags: AC_NONE, oc_flags1: OC1_NONE, oc_flags2: OC2_NONE, col_type: COL_MATERIAL_HIT3, shape: COLSHAPE_MAX }
    }
}

impl ColliderBase {
    /// `Collider_SetBase`.
    pub fn set(&mut self, init: &ColliderInit) {
        self.col_type = init.col_type;
        self.at_flags = init.at_flags;
        self.ac_flags = init.ac_flags;
        self.oc_flags1 = init.oc_flags1;
        self.oc_flags2 = init.oc_flags2;
        self.shape = init.shape;
    }
    /// `Collider_ResetATBase`.
    pub fn reset_at(&mut self) {
        self.at = None;
        self.at_flags &= !(AT_HIT | AT_BOUNCED);
    }
    /// `Collider_ResetACBase`.
    pub fn reset_ac(&mut self) {
        self.ac = None;
        self.ac_flags &= !(AC_HIT | AC_BOUNCED);
    }
    /// `Collider_ResetOCBase`.
    pub fn reset_oc(&mut self) {
        self.oc = None;
        self.oc_flags1 &= !OC1_HIT;
        self.oc_flags2 &= !OC2_HIT_PLAYER;
    }
}

/// `ColliderInit`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColliderInit {
    pub col_type: u8,
    pub at_flags: u8,
    pub ac_flags: u8,
    pub oc_flags1: u8,
    pub oc_flags2: u8,
    pub shape: u8,
}

/// `ColliderElement`: one element's attack and target properties, and this frame's hits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColliderElement {
    pub at_dmg_info: ColliderElementDamageInfoAT,
    pub ac_dmg_info: ColliderElementDamageInfoAC,
    pub elem_material: u8,
    pub at_elem_flags: u8,
    pub ac_elem_flags: u8,
    pub oc_elem_flags: u8,
    /// `atHit`, `acHit`: the collider this element hit / was hit by.
    pub at_hit: Option<ColliderRef>,
    pub ac_hit: Option<ColliderRef>,
    /// `atHitElem`, `acHitElem`: the element.
    pub at_hit_elem: Option<HitElem>,
    pub ac_hit_elem: Option<HitElem>,
}

impl Default for ColliderElement {
    /// `Collider_InitElement`.
    fn default() -> ColliderElement {
        ColliderElement {
            at_dmg_info: ColliderElementDamageInfoAT::default(),
            ac_dmg_info: ColliderElementDamageInfoAC::default(),
            elem_material: ELEM_MATERIAL_UNK0,
            at_elem_flags: ATELEM_NONE,
            ac_elem_flags: ACELEM_NONE,
            oc_elem_flags: OCELEM_NONE,
            at_hit: None,
            ac_hit: None,
            at_hit_elem: None,
            ac_hit_elem: None,
        }
    }
}

impl ColliderElement {
    /// `Collider_InitElement` + `Collider_SetElement`.
    pub fn new(init: &ColliderElementInit) -> ColliderElement {
        let mut i = ColliderElement::default();
        i.set(init);
        i
    }
    /// `Collider_SetElement`.
    pub fn set(&mut self, init: &ColliderElementInit) {
        self.elem_material = init.elem_material;
        self.at_dmg_info = init.at_dmg_info;
        self.ac_dmg_info.dmg_flags = init.ac_dmg_info.dmg_flags;
        self.ac_dmg_info.hit_backlash = init.ac_dmg_info.hit_backlash;
        self.ac_dmg_info.defense = init.ac_dmg_info.defense;
        self.at_elem_flags = init.at_elem_flags;
        self.ac_elem_flags = init.ac_elem_flags;
        self.oc_elem_flags = init.oc_elem_flags;
    }
    /// `Collider_ResetATElement`.
    pub fn reset_at(&mut self) {
        self.at_hit = None;
        self.at_hit_elem = None;
        self.at_elem_flags &= !ATELEM_HIT;
        self.at_elem_flags &= !ATELEM_DREW_HITMARK;
    }
    /// `Collider_ResetACElement`.
    pub fn reset_ac(&mut self) {
        self.ac_dmg_info.hit_pos = [0; 3];
        self.ac_elem_flags &= !ACELEM_HIT;
        self.ac_elem_flags &= !ACELEM_DRAW_HITMARK;
        self.ac_hit = None;
        self.ac_hit_elem = None;
    }
    /// `Collider_ResetOCElement`.
    pub fn reset_oc(&mut self) {
        self.oc_elem_flags &= !OCELEM_HIT;
    }
}

/// `ColliderElementInit`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColliderElementInit {
    pub elem_material: u8,
    pub at_dmg_info: ColliderElementDamageInfoAT,
    pub ac_dmg_info: ColliderElementDamageInfoACInit,
    pub at_elem_flags: u8,
    pub ac_elem_flags: u8,
    pub oc_elem_flags: u8,
}

// ---------------------------------------------------------------------------------------------
// The shapes
// ---------------------------------------------------------------------------------------------

/// `ColliderCylinder`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ColliderCylinder {
    pub base: ColliderBase,
    pub info: ColliderElement,
    pub dim: Cylinder16,
}

/// `ColliderCylinderInit`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColliderCylinderInit {
    pub base: ColliderInit,
    pub info: ColliderElementInit,
    pub dim: Cylinder16,
}

impl ColliderCylinder {
    /// `Collider_InitCylinder` + `Collider_SetCylinder` (the actor is recorded on registration).
    pub fn new(init: &ColliderCylinderInit) -> ColliderCylinder {
        let mut c = ColliderCylinder::default();
        c.base.set(&init.base);
        c.info.set(&init.info);
        c.dim = init.dim;
        c
    }
    /// `Collider_UpdateCylinder`: the cylinder at the actor's position (truncated to s16).
    pub fn update(&mut self, actor: &Actor) {
        self.dim.pos = [actor.world_pos.x as i16, actor.world_pos.y as i16, actor.world_pos.z as i16];
    }
    /// `Collider_SetCylinderPosition`.
    pub fn set_position(&mut self, pos: [i16; 3]) {
        self.dim.pos = pos;
    }
}

/// `ColliderJntSphElementDim`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ColliderJntSphElementDim {
    pub model_sphere: Sphere16,
    pub world_sphere: Sphere16,
    pub scale: f32,
    pub limb: u8,
}

/// `ColliderJntSphElement`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ColliderJntSphElement {
    pub info: ColliderElement,
    pub dim: ColliderJntSphElementDim,
}

/// `ColliderJntSphElementInit` (`ColliderJntSphElementDimInit` inlined).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColliderJntSphElementInit {
    pub info: ColliderElementInit,
    pub limb: u8,
    pub model_sphere: Sphere16,
    pub scale: i16,
}

/// `ColliderJntSph`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ColliderJntSph {
    pub base: ColliderBase,
    pub elements: Vec<ColliderJntSphElement>,
}

impl ColliderJntSph {
    /// `Collider_InitJntSph` + `Collider_SetJntSph`.
    pub fn new(base: &ColliderInit, elements: &[ColliderJntSphElementInit]) -> ColliderJntSph {
        let mut c = ColliderJntSph::default();
        c.base.set(base);
        c.elements = elements
            .iter()
            .map(|e| ColliderJntSphElement {
                info: ColliderElement::new(&e.info),
                dim: ColliderJntSphElementDim { model_sphere: e.model_sphere, world_sphere: Sphere16::default(), scale: e.scale as f32 * 0.01, limb: e.limb },
            })
            .collect();
        c
    }

    /// `Collider_UpdateSpheres`: the elements attached to `limb` move to where `mtx` (the limb's
    /// matrix, as `Matrix_MultVec3f` uses it) puts their model spheres.
    pub fn update_spheres(&mut self, limb: u8, mtx: &Mat4) {
        for e in &mut self.elements {
            if e.dim.limb == limb {
                let m = e.dim.model_sphere.center;
                let w = mtx.transform_point3(Vec3::new(m[0] as f32, m[1] as f32, m[2] as f32));
                e.dim.world_sphere.center = [w.x as i16, w.y as i16, w.z as i16];
                e.dim.world_sphere.radius = (e.dim.model_sphere.radius as f32 * e.dim.scale) as i16;
            }
        }
    }
}

/// `ColliderTrisElement`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ColliderTrisElement {
    pub info: ColliderElement,
    pub dim: TriNorm,
}

/// `ColliderTrisElementInit`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColliderTrisElementInit {
    pub info: ColliderElementInit,
    pub vtx: [Vec3; 3],
}

/// `ColliderTris`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ColliderTris {
    pub base: ColliderBase,
    pub elements: Vec<ColliderTrisElement>,
}

impl ColliderTris {
    /// `Collider_InitTris` + `Collider_SetTris` (each element's plane by `Math3D_DefPlane`).
    pub fn new(base: &ColliderInit, elements: &[ColliderTrisElementInit]) -> ColliderTris {
        let mut c = ColliderTris::default();
        c.base.set(base);
        c.elements = elements.iter().map(|e| ColliderTrisElement { info: ColliderElement::new(&e.info), dim: tris_dim(&e.vtx) }).collect();
        c
    }
    /// `Collider_SetTrisVertices`.
    pub fn set_vertices(&mut self, index: usize, a: Vec3, b: Vec3, c: Vec3) {
        self.elements[index].dim = tris_dim(&[a, b, c]);
    }
}

/// `Collider_SetTrisElementDim`: the vertices and their plane.
fn tris_dim(v: &[Vec3; 3]) -> TriNorm {
    let (normal, origin_dist) = math3d::def_plane(v[0], v[1], v[2]);
    TriNorm { vtx: *v, plane: math3d::Plane { normal, origin_dist } }
}

/// `ColliderQuadDim`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColliderQuadDim {
    pub quad: [Vec3; 4],
    /// Midpoints of d, c and of b, a.
    pub dc_mid: [i16; 3],
    pub ba_mid: [i16; 3],
    /// `acDistSq`: the nearest AC hit this frame, squared.
    pub ac_dist_sq: f32,
}

impl Default for ColliderQuadDim {
    /// `Collider_InitQuadDim`.
    fn default() -> ColliderQuadDim {
        ColliderQuadDim { quad: [Vec3::ZERO; 4], dc_mid: [0; 3], ba_mid: [0; 3], ac_dist_sq: 1.0e38 }
    }
}

impl ColliderQuadDim {
    /// `Collider_SetQuadMidpoints`.
    fn set_midpoints(&mut self) {
        let q = &self.quad;
        self.dc_mid = [((q[3].x + q[2].x) * 0.5) as i16, ((q[3].y + q[2].y) * 0.5) as i16, ((q[3].z + q[2].z) * 0.5) as i16];
        self.ba_mid = [((q[1].x + q[0].x) * 0.5) as i16, ((q[1].y + q[0].y) * 0.5) as i16, ((q[1].z + q[0].z) * 0.5) as i16];
    }
}

/// `ColliderQuad`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ColliderQuad {
    pub base: ColliderBase,
    pub info: ColliderElement,
    pub dim: ColliderQuadDim,
}

/// `ColliderQuadInit`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColliderQuadInit {
    pub base: ColliderInit,
    pub info: ColliderElementInit,
    pub quad: [Vec3; 4],
}

impl ColliderQuad {
    /// `Collider_InitQuad` + `Collider_SetQuad`.
    pub fn new(init: &ColliderQuadInit) -> ColliderQuad {
        let mut c = ColliderQuad::default();
        c.base.set(&init.base);
        c.info.set(&init.info);
        c.dim.quad = init.quad;
        c.dim.set_midpoints();
        c
    }
    /// `Collider_SetQuadVertices`.
    pub fn set_vertices(&mut self, a: Vec3, b: Vec3, c: Vec3, d: Vec3) {
        self.dim.quad = [a, b, c, d];
        self.dim.set_midpoints();
    }
}

/// What every shape offers the registration (`CollisionCheck_Set*` resets by shape).
pub trait ColliderShape {
    fn base(&self) -> &ColliderBase;
    fn base_mut(&mut self) -> &mut ColliderBase;
    /// `Collider_Reset<Shape>AT` / `AC` / `OC`.
    fn reset_at(&mut self);
    fn reset_ac(&mut self);
    fn reset_oc(&mut self);
}

impl ColliderShape for ColliderCylinder {
    fn base(&self) -> &ColliderBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut ColliderBase {
        &mut self.base
    }
    fn reset_at(&mut self) {
        self.base.reset_at();
        self.info.reset_at();
    }
    fn reset_ac(&mut self) {
        self.base.reset_ac();
        self.info.reset_ac();
    }
    fn reset_oc(&mut self) {
        self.base.reset_oc();
        self.info.reset_oc();
    }
}

impl ColliderShape for ColliderQuad {
    fn base(&self) -> &ColliderBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut ColliderBase {
        &mut self.base
    }
    /// `Collider_ResetQuadAT`: also forgets the nearest AC distance.
    fn reset_at(&mut self) {
        self.base.reset_at();
        self.info.reset_at();
        self.dim.ac_dist_sq = 1.0e38;
    }
    fn reset_ac(&mut self) {
        self.base.reset_ac();
        self.info.reset_ac();
    }
    fn reset_oc(&mut self) {
        self.base.reset_oc();
        self.info.reset_oc();
    }
}

impl ColliderShape for ColliderJntSph {
    fn base(&self) -> &ColliderBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut ColliderBase {
        &mut self.base
    }
    fn reset_at(&mut self) {
        self.base.reset_at();
        self.elements.iter_mut().for_each(|e| e.info.reset_at());
    }
    fn reset_ac(&mut self) {
        self.base.reset_ac();
        self.elements.iter_mut().for_each(|e| e.info.reset_ac());
    }
    fn reset_oc(&mut self) {
        self.base.reset_oc();
        self.elements.iter_mut().for_each(|e| e.info.reset_oc());
    }
}

impl ColliderShape for ColliderTris {
    fn base(&self) -> &ColliderBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut ColliderBase {
        &mut self.base
    }
    fn reset_at(&mut self) {
        self.base.reset_at();
        self.elements.iter_mut().for_each(|e| e.info.reset_at());
    }
    fn reset_ac(&mut self) {
        self.base.reset_ac();
        self.elements.iter_mut().for_each(|e| e.info.reset_ac());
    }
    fn reset_oc(&mut self) {
        self.base.reset_oc();
        self.elements.iter_mut().for_each(|e| e.info.reset_oc());
    }
}

/// A collider of any shape, owned: what the checks run over.
#[derive(Debug, Clone, PartialEq)]
pub enum Collider {
    JntSph(ColliderJntSph),
    Cylinder(ColliderCylinder),
    Tris(ColliderTris),
    Quad(ColliderQuad),
}

/// A collider of any shape, as an actor lends it (`ActorImpl::collider_mut`).
pub enum ColliderMut<'a> {
    JntSph(&'a mut ColliderJntSph),
    Cylinder(&'a mut ColliderCylinder),
    Tris(&'a mut ColliderTris),
    Quad(&'a mut ColliderQuad),
}

impl ColliderMut<'_> {
    /// Takes the collider out, leaving a default one behind until `put`.
    fn take(self) -> Collider {
        match self {
            ColliderMut::JntSph(c) => Collider::JntSph(std::mem::take(c)),
            ColliderMut::Cylinder(c) => Collider::Cylinder(std::mem::take(c)),
            ColliderMut::Tris(c) => Collider::Tris(std::mem::take(c)),
            ColliderMut::Quad(c) => Collider::Quad(std::mem::take(c)),
        }
    }
    /// Puts back what `take` took.
    fn put(self, col: Collider) {
        match (self, col) {
            (ColliderMut::JntSph(c), Collider::JntSph(v)) => *c = v,
            (ColliderMut::Cylinder(c), Collider::Cylinder(v)) => *c = v,
            (ColliderMut::Tris(c), Collider::Tris(v)) => *c = v,
            (ColliderMut::Quad(c), Collider::Quad(v)) => *c = v,
            _ => log::error!("collision check: a collider changed shape while it was checked"),
        }
    }
}

impl Collider {
    pub fn base(&self) -> &ColliderBase {
        match self {
            Collider::JntSph(c) => &c.base,
            Collider::Cylinder(c) => &c.base,
            Collider::Tris(c) => &c.base,
            Collider::Quad(c) => &c.base,
        }
    }
    pub fn base_mut(&mut self) -> &mut ColliderBase {
        match self {
            Collider::JntSph(c) => &mut c.base,
            Collider::Cylinder(c) => &mut c.base,
            Collider::Tris(c) => &mut c.base,
            Collider::Quad(c) => &mut c.base,
        }
    }
    fn shape(&self) -> u8 {
        match self {
            Collider::JntSph(_) => COLSHAPE_JNTSPH,
            Collider::Cylinder(_) => COLSHAPE_CYLINDER,
            Collider::Tris(_) => COLSHAPE_TRIS,
            Collider::Quad(_) => COLSHAPE_QUAD,
        }
    }
    /// Element `i`'s info.
    pub fn info(&self, i: usize) -> Option<&ColliderElement> {
        match self {
            Collider::JntSph(c) => c.elements.get(i).map(|e| &e.info),
            Collider::Tris(c) => c.elements.get(i).map(|e| &e.info),
            Collider::Cylinder(c) => (i == 0).then_some(&c.info),
            Collider::Quad(c) => (i == 0).then_some(&c.info),
        }
    }
    pub fn info_mut(&mut self, i: usize) -> Option<&mut ColliderElement> {
        match self {
            Collider::JntSph(c) => c.elements.get_mut(i).map(|e| &mut e.info),
            Collider::Tris(c) => c.elements.get_mut(i).map(|e| &mut e.info),
            Collider::Cylinder(c) => (i == 0).then_some(&mut c.info),
            Collider::Quad(c) => (i == 0).then_some(&mut c.info),
        }
    }
    fn element_count(&self) -> usize {
        match self {
            Collider::JntSph(c) => c.elements.len(),
            Collider::Tris(c) => c.elements.len(),
            _ => 1,
        }
    }
}

// ---------------------------------------------------------------------------------------------
// The context
// ---------------------------------------------------------------------------------------------

/// `CollisionCheckContext`: this frame's registered colliders.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CollisionCheckContext {
    pub col_at: Vec<ColliderRef>,
    pub col_ac: Vec<ColliderRef>,
    pub col_oc: Vec<ColliderRef>,
}

impl CollisionCheckContext {
    /// `CollisionCheck_ClearContext` (without SAC mode).
    pub fn clear(&mut self) {
        self.col_at.clear();
        self.col_ac.clear();
        self.col_oc.clear();
    }

    /// The shared part of `CollisionCheck_SetAT` / `SetAC` / `SetOC` after the reset: the owner
    /// must still be alive (`actor->update != NULL`) and the list not full. Returns the index.
    fn register(list: &mut Vec<ColliderRef>, max: usize, owner: Option<ActorHandle>, owner_actor: &Actor, base: &mut ColliderBase, id: u8, what: &str) -> i32 {
        let Some(h) = owner else {
            log::warn!("CollisionCheck_Set{what}: a collider registered outside an actor's update or draw");
            return -1;
        };
        base.actor = Some(h);
        if owner_actor.killed {
            return -1;
        }
        if list.len() >= max {
            log::warn!("CollisionCheck_Set{what}(): index exceeded and cannot add more");
            return -1;
        }
        list.push(ColliderRef { actor: h, id });
        list.len() as i32 - 1
    }

    /// `CollisionCheck_SetAT`: `c` (collider `id` of `owner`) attacks this frame.
    pub fn set_at(&mut self, owner: Option<ActorHandle>, owner_actor: &Actor, id: u8, c: &mut impl ColliderShape) -> i32 {
        c.reset_at();
        Self::register(&mut self.col_at, COLLISION_CHECK_AT_MAX, owner, owner_actor, c.base_mut(), id, "AT")
    }

    /// `CollisionCheck_SetAC`: `c` can be hit this frame.
    pub fn set_ac(&mut self, owner: Option<ActorHandle>, owner_actor: &Actor, id: u8, c: &mut impl ColliderShape) -> i32 {
        c.reset_ac();
        Self::register(&mut self.col_ac, COLLISION_CHECK_AC_MAX, owner, owner_actor, c.base_mut(), id, "AC")
    }

    /// `CollisionCheck_SetOC`: `c` pushes and is pushed this frame.
    pub fn set_oc(&mut self, owner: Option<ActorHandle>, owner_actor: &Actor, id: u8, c: &mut impl ColliderShape) -> i32 {
        c.reset_oc();
        Self::register(&mut self.col_oc, COLLISION_CHECK_OC_MAX, owner, owner_actor, c.base_mut(), id, "OC")
    }

    /// The registered OC colliders' shapes, in list order, for `CollisionCheck_LineOCCheck`.
    pub fn oc_lines(&self, actors: &mut ActorContext) -> OcLines {
        let mut out = Vec::new();
        for &r in &self.col_oc {
            let Some(cm) = actors.get_mut(r.actor).and_then(|a| a.collider_mut(r.id)) else { continue };
            let shape = match cm {
                ColliderMut::JntSph(c) => OcLine { actor: c.base.actor, on: c.base.oc_flags1 & OC1_ON != 0, shape: OcShape::JntSph(c.elements.iter().map(|e| (e.info.oc_elem_flags & OCELEM_ON != 0, e.dim.world_sphere)).collect()) },
                ColliderMut::Cylinder(c) => OcLine { actor: c.base.actor, on: c.base.oc_flags1 & OC1_ON != 0, shape: OcShape::Cylinder(c.info.oc_elem_flags & OCELEM_ON != 0, c.dim) },
                ColliderMut::Tris(c) => OcLine { actor: c.base.actor, on: c.base.oc_flags1 & OC1_ON != 0, shape: OcShape::Other },
                ColliderMut::Quad(c) => OcLine { actor: c.base.actor, on: c.base.oc_flags1 & OC1_ON != 0, shape: OcShape::Other },
            };
            out.push(shape);
        }
        OcLines(out)
    }

    /// `CollisionCheck_AT`, `CollisionCheck_OC` and `CollisionCheck_Damage` over the registered
    /// colliders, as `Play_Update` runs them. The colliders are taken out of their actors
    /// for the checks (`ActorImpl::collider_mut`) and put back after.
    /// Returns the hit effects (`CollisionCheck_HitEffects`: the sounds, the hit marks, the
    /// blood, the shield's particles), in order, for the play state to run.
    pub fn check(&self, actors: &mut ActorContext) -> Vec<HitFx> {
        let mut set = ColliderSet::take(self, actors);
        set.at(actors);
        set.oc(actors);
        set.damage(actors);
        let hit_fx = std::mem::take(&mut set.hit_fx);
        set.put_back(actors);
        hit_fx
    }
}

/// One of `CollisionCheck_HitEffects`' calls, in its order: run by the play state after the
/// checks (`PlayState::collision_check_hit_fx`), since they need the effects and the audio.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HitFx {
    /// `Audio_PlaySfxGeneral(id, pos, 4, ...)` with the defaults (`SFX_PLAY_AT_POS`,
    /// `SFX_PLAY_CENTERED`).
    Sfx(u16, SfxPos),
    /// `EffectSsHitMark_SpawnFixedScale(play, type, hitPos)`.
    HitMark(i32, Vec3),
    /// `sBloodFuncs[blood]` at the hit (`BLOOD_*`: 1 blue, 2 green, 3 water, 4 and 5 red).
    Blood(u8, Vec3),
    /// `CollisionCheck_SpawnShieldParticles` (metal's: with its light).
    ShieldParticlesMetal(Vec3),
    /// `CollisionCheck_SpawnShieldParticlesWood`'s particles (no light; its sound is a `Sfx`).
    ShieldParticlesWood(Vec3),
}

/// `ColChkBloodType` (`z_collision_check.c`).
pub const BLOOD_NONE: u8 = 0;
pub const BLOOD_BLUE: u8 = 1;
pub const BLOOD_GREEN: u8 = 2;
pub const BLOOD_WATER: u8 = 3;
pub const BLOOD_RED: u8 = 4;
pub const BLOOD_RED2: u8 = 5;

/// An OC collider's shape, as `CollisionCheck_LineOC` sees it.
#[derive(Debug, Clone, PartialEq)]
pub enum OcShape {
    /// The spheres, each with `OCELEM_ON`.
    JntSph(Vec<(bool, Sphere16)>),
    /// `OCELEM_ON`, and the cylinder.
    Cylinder(bool, Cylinder16),
    /// Triangles and quads (`sOCLineCheckFuncs` has no check for them).
    Other,
}

/// A registered OC collider: its actor, `OC1_ON`, its shape.
#[derive(Debug, Clone, PartialEq)]
pub struct OcLine {
    pub actor: Option<ActorHandle>,
    pub on: bool,
    pub shape: OcShape,
}

/// The OC list's colliders (`CollisionCheckContext::oc_lines`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OcLines(pub Vec<OcLine>);

impl OcLines {
    /// `CollisionCheck_LineOCCheck`: does the segment `a`-`b` cross an OC collider of an actor
    /// not in `exclusions` (`CollisionCheck_LineOC_JntSph`, `_Cyl`)?
    pub fn line_oc_check(&self, a: Vec3, b: Vec3, exclusions: &[Option<ActorHandle>]) -> bool {
        for c in &self.0 {
            // CollisionCheck_SkipOC.
            if !c.on || exclusions.contains(&c.actor) {
                continue;
            }
            let hit = match &c.shape {
                OcShape::JntSph(spheres) => spheres.iter().any(|(on, s)| *on && math3d::line_vs_sph(s, &math3d::Linef { a, b })),
                OcShape::Cylinder(on, cyl) => *on && math3d::cyl_vs_line_seg(cyl, a, b).0 != 0,
                OcShape::Other => false,
            };
            if hit {
                return true;
            }
        }
        false
    }
}

/// The registered colliders, owned for the checks: each distinct `ColliderRef` once, and the
/// three lists as indices into them.
struct ColliderSet {
    refs: Vec<ColliderRef>,
    cols: Vec<Option<Collider>>,
    at: Vec<usize>,
    ac: Vec<usize>,
    oc: Vec<usize>,
    /// The hit effects, in order.
    hit_fx: Vec<HitFx>,
}

impl ColliderSet {
    fn take(ctx: &CollisionCheckContext, actors: &mut ActorContext) -> ColliderSet {
        let mut s = ColliderSet { refs: Vec::new(), cols: Vec::new(), at: Vec::new(), ac: Vec::new(), oc: Vec::new(), hit_fx: Vec::new() };
        let index = |s: &mut ColliderSet, r: ColliderRef, actors: &mut ActorContext| -> usize {
            if let Some(i) = s.refs.iter().position(|x| *x == r) {
                return i;
            }
            let col = actors.get_mut(r.actor).and_then(|a| a.collider_mut(r.id)).map(|c| c.take());
            s.refs.push(r);
            s.cols.push(col);
            s.refs.len() - 1
        };
        for &r in &ctx.col_at {
            let i = index(&mut s, r, actors);
            s.at.push(i);
        }
        for &r in &ctx.col_ac {
            let i = index(&mut s, r, actors);
            s.ac.push(i);
        }
        for &r in &ctx.col_oc {
            let i = index(&mut s, r, actors);
            s.oc.push(i);
        }
        s
    }

    fn put_back(self, actors: &mut ActorContext) {
        for (r, col) in self.refs.into_iter().zip(self.cols) {
            if let Some(col) = col
                && let Some(cm) = actors.get_mut(r.actor).and_then(|a| a.collider_mut(r.id))
            {
                cm.put(col);
            }
        }
    }

    fn index_of(&self, r: ColliderRef) -> Option<usize> {
        self.refs.iter().position(|x| *x == r)
    }

    /// `actor->update == NULL` for a collider's actor (`Actor_Kill` ran, or it's gone).
    fn actor_dead(actors: &ActorContext, base: &ColliderBase) -> bool {
        base.actor.is_some_and(|h| actors.actor(h).is_none_or(|a| a.killed))
    }

    // ---- AT vs AC ------------------------------------------------------------------------

    /// `CollisionCheck_AT` (with `CollisionCheck_AC` inlined).
    fn at(&mut self, actors: &mut ActorContext) {
        if self.at.is_empty() || self.ac.is_empty() {
            return;
        }
        for k in 0..self.at.len() {
            let i = self.at[k];
            let Some(at) = &self.cols[i] else { continue };
            if at.base().at_flags & AT_ON == 0 || Self::actor_dead(actors, at.base()) {
                continue;
            }
            // CollisionCheck_AC.
            for m in 0..self.ac.len() {
                let j = self.ac[m];
                let (Some(at), Some(ac)) = (&self.cols[i], &self.cols[j]) else { continue };
                if ac.base().ac_flags & AC_ON == 0 || Self::actor_dead(actors, ac.base()) {
                    continue;
                }
                if ac.base().ac_flags & at.base().at_flags & AC_TYPE_ALL != 0 && i != j {
                    if at.base().at_flags & AT_SELF == 0 && at.base().actor.is_some() && ac.base().actor == at.base().actor {
                        continue;
                    }
                    self.at_vs_ac(i, j, actors);
                }
            }
        }
        self.set_hit_effects(actors);
    }

    /// `sACVsFuncs[at->shape][ac->shape]`, with the two colliders taken out of the set.
    fn at_vs_ac(&mut self, i: usize, j: usize, actors: &mut ActorContext) {
        let (Some(mut at), Some(mut ac)) = (self.cols[i].take(), self.cols[j].take()) else { return };
        let (ri, rj) = (self.refs[i], self.refs[j]);
        let mut cx = PairCtx { set: self, actors, ri, rj, reset_ac_of_j: None };
        match (&mut at, &mut ac) {
            (Collider::JntSph(a), Collider::JntSph(b)) => cx.jntsph_vs_jntsph(a, b),
            (Collider::JntSph(a), Collider::Cylinder(b)) => cx.jntsph_vs_cyl(a, b),
            (Collider::Cylinder(a), Collider::JntSph(b)) => cx.cyl_vs_jntsph(a, b),
            (Collider::JntSph(a), Collider::Tris(b)) => cx.jntsph_vs_tris(a, b),
            (Collider::Tris(a), Collider::JntSph(b)) => cx.tris_vs_jntsph(a, b),
            (Collider::JntSph(a), Collider::Quad(b)) => cx.jntsph_vs_quad(a, b),
            (Collider::Quad(a), Collider::JntSph(b)) => cx.quad_vs_jntsph(a, b),
            (Collider::Cylinder(a), Collider::Cylinder(b)) => cx.cyl_vs_cyl(a, b),
            (Collider::Cylinder(a), Collider::Tris(b)) => cx.cyl_vs_tris(a, b),
            (Collider::Tris(a), Collider::Cylinder(b)) => cx.tris_vs_cyl(a, b),
            (Collider::Cylinder(a), Collider::Quad(b)) => cx.cyl_vs_quad(a, b),
            (Collider::Quad(a), Collider::Cylinder(b)) => cx.quad_vs_cyl(a, b),
            (Collider::Tris(a), Collider::Tris(b)) => cx.tris_vs_tris(a, b),
            (Collider::Tris(a), Collider::Quad(b)) => cx.tris_vs_quad(a, b),
            (Collider::Quad(a), Collider::Tris(b)) => cx.quad_vs_tris(a, b),
            (Collider::Quad(a), Collider::Quad(b)) => cx.quad_vs_quad(a, b),
        }
        // `Collider_QuadSetNearestAC` may have reset the AC collider it had hit before: when
        // that was this pair's AC collider, the reset was deferred to here.
        if let Some(reset) = cx.reset_ac_of_j.take() {
            apply_ac_reset(&mut ac, reset);
        }
        self.cols[i] = Some(at);
        self.cols[j] = Some(ac);
    }

    /// `CollisionCheck_SetHitEffects`: each AC collider's first element hit this frame whose
    /// mark wasn't drawn yet has its hit effects (`CollisionCheck_SetJntSphHitFX` and the rest).
    fn set_hit_effects(&mut self, actors: &ActorContext) {
        for k in 0..self.ac.len() {
            let j = self.ac[k];
            let Some(ac) = &self.cols[j] else { continue };
            if ac.base().ac_flags & AC_ON == 0 || Self::actor_dead(actors, ac.base()) {
                continue;
            }
            // Cylinders and quads have one element; JntSph and Tris stop at the first mark.
            for e in 0..ac.element_count() {
                let Some(info) = self.cols[j].as_ref().and_then(|c| c.info(e)) else { break };
                let Some(hit) = info.ac_hit_elem else { continue };
                if info.ac_elem_flags & ACELEM_DRAW_HITMARK == 0 {
                    continue;
                }
                let (ac_elem_flags, elem_material) = (info.ac_elem_flags, info.elem_material);
                let Some(ai) = self.index_of(hit.elem.col) else { continue };
                let Some(at_info) = self.cols[ai].as_ref().and_then(|c| c.info(hit.elem.elem as usize)) else { continue };
                if at_info.at_elem_flags & ATELEM_DREW_HITMARK != 0 {
                    continue;
                }
                let at_elem_flags = at_info.at_elem_flags;
                // Math_Vec3s_ToVec3f(&hitPos, &elem->acDmgInfo.hitPos).
                let hit_pos = v3s(info.ac_dmg_info.hit_pos);
                let (Some(at), Some(ac)) = (self.cols[ai].as_ref(), self.cols[j].as_ref()) else { continue };
                hit_effects(&mut self.hit_fx, actors, at.base(), at_elem_flags, ac.base(), ac_elem_flags, elem_material, hit_pos);
                if let Some(at_info) = self.cols[ai].as_mut().and_then(|c| c.info_mut(hit.elem.elem as usize)) {
                    at_info.at_elem_flags |= ATELEM_DREW_HITMARK;
                }
                break;
            }
        }
    }

    // ---- OC --------------------------------------------------------------------------------

    /// `CollisionCheck_OC`: every OC collider against every later one.
    fn oc(&mut self, actors: &mut ActorContext) {
        for a in 0..self.oc.len() {
            let l = self.oc[a];
            if self.cols[l].as_ref().is_none_or(|c| c.base().oc_flags1 & OC1_ON == 0) {
                continue;
            }
            for b in a + 1..self.oc.len() {
                let r = self.oc[b];
                let (Some(left), Some(right)) = (&self.cols[l], &self.cols[r]) else { continue };
                if right.base().oc_flags1 & OC1_ON == 0 || incompatible(left.base(), right.base()) {
                    continue;
                }
                if l == r {
                    // The same collider registered twice: CollisionCheck_Incompatible's
                    // same-actor rule already skips it.
                    continue;
                }
                let (Some(mut left), Some(mut right)) = (self.cols[l].take(), self.cols[r].take()) else { continue };
                let (rl, rr) = (self.refs[l], self.refs[r]);
                match (&mut left, &mut right) {
                    (Collider::JntSph(x), Collider::JntSph(y)) => oc_jntsph_vs_jntsph(x, rl, y, rr, actors),
                    (Collider::JntSph(x), Collider::Cylinder(y)) => oc_jntsph_vs_cyl(x, rl, y, rr, actors),
                    // CollisionCheck_OC_CylVsJntSph: the same, the other way round.
                    (Collider::Cylinder(x), Collider::JntSph(y)) => oc_jntsph_vs_cyl(y, rr, x, rl, actors),
                    (Collider::Cylinder(x), Collider::Cylinder(y)) => oc_cyl_vs_cyl(x, rl, y, rr, actors),
                    (x, y) => log::debug!("CollisionCheck_OC(): not compatible {}, {}", x.shape(), y.shape()),
                }
                self.cols[l] = Some(left);
                self.cols[r] = Some(right);
            }
        }
    }

    // ---- Damage ----------------------------------------------------------------------------

    /// `CollisionCheck_Damage`: each AC collider's hits, by `sApplyDamageFuncs[col->shape]`.
    fn damage(&mut self, actors: &mut ActorContext) {
        for k in 0..self.ac.len() {
            let j = self.ac[k];
            let Some(c) = &self.cols[j] else { continue };
            if c.base().ac_flags & AC_NO_DAMAGE != 0 {
                continue;
            }
            match c {
                Collider::JntSph(c) => apply_damage_jnt_sph(c, actors),
                Collider::Cylinder(c) => apply_damage_cyl(c, actors),
                Collider::Tris(c) => apply_damage_tris(c, actors),
                Collider::Quad(c) => apply_damage_quad(c, actors),
            }
        }
    }
}

/// `CollisionCheck_ApplyDamageJntSph`: every sphere's hit.
fn apply_damage_jnt_sph(c: &ColliderJntSph, actors: &mut ActorContext) {
    for e in &c.elements {
        apply_damage(&c.base, &e.info, actors);
    }
}

/// `CollisionCheck_ApplyDamageCyl`.
fn apply_damage_cyl(c: &ColliderCylinder, actors: &mut ActorContext) {
    apply_damage(&c.base, &c.info, actors);
}

/// `CollisionCheck_ApplyDamageTris`: every triangle's hit.
fn apply_damage_tris(c: &ColliderTris, actors: &mut ActorContext) {
    for e in &c.elements {
        apply_damage(&c.base, &e.info, actors);
    }
}

/// `CollisionCheck_ApplyDamageQuad`.
fn apply_damage_quad(c: &ColliderQuad, actors: &mut ActorContext) {
    apply_damage(&c.base, &c.info, actors);
}

/// `CollisionCheck_ApplyDamage`: a hit element's damage to its actor's `colChkInfo`. With a
/// damage table, the entry for the attack's damage flag (its reaction into `damageReaction`);
/// without one, the attack's damage less the element's defence. A hard collider (`AC_HARD`)
/// takes none.
pub fn apply_damage(base: &ColliderBase, info: &ColliderElement, actors: &mut ActorContext) {
    let Some(h) = base.actor else { return };
    if base.ac_flags & AC_HIT == 0 {
        return;
    }
    if info.ac_elem_flags & ACELEM_HIT == 0 || info.ac_elem_flags & ACELEM_NO_DAMAGE != 0 {
        return;
    }
    let Some(hit) = info.ac_hit_elem else {
        log::error!("pclobj_elem->ac_hit_elem != NULL");
        return;
    };
    let Some(actor) = actors.actor_mut(h) else { return };
    let damage = match actor.col_chk_info.damage_table {
        None => (hit.at_dmg_info.damage as f32 - info.ac_dmg_info.defense as f32).max(0.0),
        Some(tbl) => {
            let entry = tbl.lookup(hit.at_dmg_info.dmg_flags);
            actor.col_chk_info.damage_reaction = (entry >> 4) & 0xF;
            (entry & 0xF) as f32
        }
    };
    if base.ac_flags & AC_HARD == 0 {
        actor.col_chk_info.damage = (actor.col_chk_info.damage as f32 + damage) as u8;
    }
}

/// A deferred `Collider_ResetACBase` / `Collider_ResetACElement` on the pair's AC collider.
#[derive(Debug, Clone, Copy)]
struct AcReset {
    base: bool,
    elem: Option<u16>,
}

fn apply_ac_reset(c: &mut Collider, r: AcReset) {
    if r.base {
        c.base_mut().reset_ac();
    }
    if let Some(e) = r.elem
        && let Some(info) = c.info_mut(e as usize)
    {
        info.reset_ac();
    }
}

/// One AT-vs-AC pair being checked: the rest of the set (for `Collider_QuadSetNearestAC`'s
/// resets of an earlier hit) and the actors (the hit effects on `colChkInfo`).
struct PairCtx<'a> {
    set: &'a mut ColliderSet,
    actors: &'a mut ActorContext,
    ri: ColliderRef,
    rj: ColliderRef,
    reset_ac_of_j: Option<AcReset>,
}

/// `sHitInfo`'s effects by `colType` (`HIT_*`: 0 white, 1 dust, 2 red, 3 solid, 4 wood, 5
/// none).
const HIT_INFO_EFFECT: [u8; 14] = [0, 1, 1, 0, 5, 2, 0, 0, 2, 3, 5, 3, 3, 4];
/// `sHitInfo`'s blood by `colType` (`BLOOD_*`).
const HIT_INFO_BLOOD: [u8; 14] = [BLOOD_BLUE, BLOOD_NONE, BLOOD_GREEN, BLOOD_NONE, BLOOD_WATER, BLOOD_NONE, BLOOD_GREEN, BLOOD_RED, BLOOD_BLUE, BLOOD_NONE, BLOOD_NONE, BLOOD_NONE, BLOOD_NONE, BLOOD_NONE];
const HIT_SOLID: u8 = 3;
const HIT_WOOD: u8 = 4;
const HIT_NONE: u8 = 5;
const ATELEM_SFX_MASK: u8 = 3 << 3;

/// Where `&collider->actor->projectedPos` (or `gSfxDefaultPos` with no actor) is.
fn actor_sfx_pos(actor: Option<ActorHandle>) -> SfxPos {
    actor.map(SfxPos::Actor).unwrap_or(SfxPos::Default)
}

/// `CollisionCheck_HitEffects`, by the AC collider's `colType` (`sHitInfo`): its blood, then
/// a solid's (`CollisionCheck_HitSolid`), wood's (the shield particles) or the hit mark with the
/// sword's sound; with no actor, the white mark and the shield's bounce. `at`'s element's
/// `at_elem_flags`, `ac`'s element's `ac_elem_flags` and `elem_material`, at `hit_pos`.
#[allow(clippy::too_many_arguments)]
fn hit_effects(out: &mut Vec<HitFx>, actors: &ActorContext, at: &ColliderBase, at_elem_flags: u8, ac: &ColliderBase, ac_elem_flags: u8, elem_material: u8, hit_pos: Vec3) {
    use crate::audio::sfx::{NA_SE_IT_REFLECTION_WOOD, NA_SE_IT_SHIELD_BOUND};
    use crate::effect::hitmark::EFFECT_HITMARK_WHITE;
    if ac_elem_flags & ACELEM_NO_HITMARK != 0 {
        return;
    }
    if at_elem_flags & ATELEM_AT_HITMARK == 0 && at_elem_flags & ATELEM_DREW_HITMARK != 0 {
        return;
    }
    if ac.actor.is_some() {
        // sBloodFuncs[sHitInfo[acCol->colMaterial].blood].
        let blood = HIT_INFO_BLOOD.get(ac.col_type as usize).copied().unwrap_or(BLOOD_NONE);
        if blood != BLOOD_NONE {
            out.push(HitFx::Blood(blood, hit_pos));
        }
        match HIT_INFO_EFFECT.get(ac.col_type as usize).copied().unwrap_or(HIT_NONE) {
            HIT_SOLID => hit_solid(out, at_elem_flags, ac, hit_pos),
            HIT_WOOD => {
                if at.actor.is_none() {
                    out.push(HitFx::ShieldParticlesMetal(hit_pos));
                    out.push(HitFx::Sfx(NA_SE_IT_REFLECTION_WOOD, SfxPos::Default));
                } else {
                    // CollisionCheck_SpawnShieldParticlesWood.
                    out.push(HitFx::ShieldParticlesWood(hit_pos));
                    out.push(HitFx::Sfx(NA_SE_IT_REFLECTION_WOOD, actor_sfx_pos(at.actor)));
                }
            }
            HIT_NONE => {}
            effect => {
                out.push(HitFx::HitMark(effect as i32, hit_pos));
                if ac_elem_flags & ACELEM_NO_SWORD_SFX == 0 {
                    sword_hit_audio(out, actors, at, elem_material);
                }
            }
        }
    } else {
        out.push(HitFx::HitMark(EFFECT_HITMARK_WHITE, hit_pos));
        out.push(HitFx::Sfx(NA_SE_IT_SHIELD_BOUND, actor_sfx_pos(ac.actor)));
    }
}

/// `CollisionCheck_HitSolid` (METAL, WOOD, HARD and TREE AC colliders), by the AT element's
/// `ATELEM_SFX_*`: the white mark and the shield's bounce; metal's mark, particles and
/// `NA_SE_IT_SHIELD_REFLECT_SW` (`CollisionCheck_SpawnShieldParticlesMetal(Sfx)`); the dust mark
/// and wood's sound.
fn hit_solid(out: &mut Vec<HitFx>, at_elem_flags: u8, collider: &ColliderBase, hit_pos: Vec3) {
    use crate::audio::sfx::{NA_SE_IT_REFLECTION_WOOD, NA_SE_IT_SHIELD_BOUND, NA_SE_IT_SHIELD_REFLECT_SW};
    use crate::effect::hitmark::{EFFECT_HITMARK_DUST, EFFECT_HITMARK_METAL, EFFECT_HITMARK_WHITE};
    let flags = at_elem_flags & ATELEM_SFX_MASK;
    let pos = actor_sfx_pos(collider.actor);
    if flags == ATELEM_SFX_NORMAL && collider.col_type != COL_MATERIAL_METAL {
        out.push(HitFx::HitMark(EFFECT_HITMARK_WHITE, hit_pos));
        out.push(HitFx::Sfx(NA_SE_IT_SHIELD_BOUND, pos));
    } else if flags == ATELEM_SFX_NORMAL {
        out.push(HitFx::HitMark(EFFECT_HITMARK_METAL, hit_pos));
        out.push(HitFx::ShieldParticlesMetal(hit_pos));
        out.push(HitFx::Sfx(NA_SE_IT_SHIELD_REFLECT_SW, pos));
    } else if flags == ATELEM_SFX_HARD {
        out.push(HitFx::HitMark(EFFECT_HITMARK_WHITE, hit_pos));
        out.push(HitFx::Sfx(NA_SE_IT_SHIELD_BOUND, pos));
    } else if flags == ATELEM_SFX_WOOD {
        out.push(HitFx::HitMark(EFFECT_HITMARK_DUST, hit_pos));
        out.push(HitFx::Sfx(NA_SE_IT_REFLECTION_WOOD, pos));
    }
}

/// `CollisionCheck_SwordHitAudio`: a Player-attached AT collider's strike, by the AC element's
/// `elemType`.
fn sword_hit_audio(out: &mut Vec<HitFx>, actors: &ActorContext, at: &ColliderBase, elem_material: u8) {
    use crate::audio::sfx::{NA_SE_IT_SWORD_STRIKE, NA_SE_IT_SWORD_STRIKE_HARD, NA_SE_PL_WALK_GROUND, SFX_FLAG};
    let Some(h) = at.actor else { return };
    if actors.actor(h).is_some_and(|a| a.category == crate::actor_ctx::ACTORCAT_PLAYER) {
        let id = match elem_material {
            ELEM_MATERIAL_UNK0 => NA_SE_IT_SWORD_STRIKE,
            ELEM_MATERIAL_UNK1 => NA_SE_IT_SWORD_STRIKE_HARD,
            ELEM_MATERIAL_UNK2 | ELEM_MATERIAL_UNK3 => NA_SE_PL_WALK_GROUND - SFX_FLAG,
            _ => return,
        };
        out.push(HitFx::Sfx(id, SfxPos::Actor(h)));
    }
}

/// `Math_Vec3s_ToVec3f`.
fn v3s(v: [i16; 3]) -> Vec3 {
    Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32)
}

/// `CollisionCheck_IsElementNotAT`.
fn skip_touch(i: &ColliderElement) -> bool {
    i.at_elem_flags & ATELEM_ON == 0
}

/// `CollisionCheck_IsElementNotAC`.
fn skip_bump(i: &ColliderElement) -> bool {
    i.ac_elem_flags & ACELEM_ON == 0
}

/// `CollisionCheck_NoSharedFlags`.
fn no_shared_flags(at: &ColliderElement, ac: &ColliderElement) -> bool {
    at.at_dmg_info.dmg_flags & ac.ac_dmg_info.dmg_flags == 0
}

/// The quad's two triangles as the AT side builds them: (2, 3, 1) and (2, 1, 0).
fn quad_tris_at(d: &ColliderQuadDim) -> [TriNorm; 2] {
    [math3d::tri_norm(d.quad[2], d.quad[3], d.quad[1]), math3d::tri_norm(d.quad[2], d.quad[1], d.quad[0])]
}

/// The quad's two triangles as the AC side builds them: (2, 3, 1) and (1, 0, 2).
fn quad_tris_ac(d: &ColliderQuadDim) -> [TriNorm; 2] {
    [math3d::tri_norm(d.quad[2], d.quad[3], d.quad[1]), math3d::tri_norm(d.quad[1], d.quad[0], d.quad[2])]
}

impl PairCtx<'_> {
    /// `CollisionCheck_SetATvsAC`: records the hit on both sides. (The C also takes the two
    /// elements' centres, which it doesn't use; the checks here don't compute them.)
    #[allow(clippy::too_many_arguments)]
    fn set_at_vs_ac(&mut self, at: &mut ColliderBase, at_info: &mut ColliderElement, at_elem: u16, ac: &mut ColliderBase, ac_info: &mut ColliderElement, ac_elem: u16, hit_pos: Vec3) {
        if ac.ac_flags & AC_HARD != 0 && at.actor.is_some() && ac.actor.is_some() {
            // CollisionCheck_SetBounce.
            at.at_flags |= AT_BOUNCED;
            ac.ac_flags |= AC_BOUNCED;
        }
        let at_ref = ElemRef { col: self.ri, elem: at_elem };
        let ac_ref = ElemRef { col: self.rj, elem: ac_elem };
        if ac_info.ac_elem_flags & ACELEM_NO_AT_INFO == 0 {
            at.at_flags |= AT_HIT;
            at.at = ac.actor;
            at_info.at_hit = Some(self.rj);
            at_info.at_hit_elem = Some(HitElem { elem: ac_ref, at_dmg_info: ac_info.at_dmg_info, ac_dmg_info: ac_info.ac_dmg_info, elem_material: ac_info.elem_material });
            at_info.at_elem_flags |= ATELEM_HIT;
            if let Some(a) = at.actor.and_then(|h| self.actors.actor_mut(h)) {
                a.col_chk_info.at_hit_backlash = ac_info.ac_dmg_info.hit_backlash;
            }
        }
        ac.ac_flags |= AC_HIT;
        ac.ac = at.actor;
        ac_info.ac_hit = Some(self.ri);
        ac_info.ac_hit_elem = Some(HitElem { elem: at_ref, at_dmg_info: at_info.at_dmg_info, ac_dmg_info: at_info.ac_dmg_info, elem_material: at_info.elem_material });
        ac_info.ac_elem_flags |= ACELEM_HIT;
        if let Some(a) = ac.actor.and_then(|h| self.actors.actor_mut(h)) {
            a.col_chk_info.ac_hit_special_effect = at_info.at_dmg_info.hit_special_effect;
        }
        ac_info.ac_dmg_info.hit_pos = [hit_pos.x as i16, hit_pos.y as i16, hit_pos.z as i16];
        if at_info.at_elem_flags & ATELEM_AT_HITMARK == 0 && ac.col_type != COL_MATERIAL_METAL && ac.col_type != COL_MATERIAL_WOOD && ac.col_type != COL_MATERIAL_HARD {
            ac_info.ac_elem_flags |= ACELEM_DRAW_HITMARK;
        } else {
            hit_effects(&mut self.set.hit_fx, self.actors, at, at_info.at_elem_flags, ac, ac_info.ac_elem_flags, ac_info.elem_material, hit_pos);
            at_info.at_elem_flags |= ATELEM_DREW_HITMARK;
        }
    }

    /// `Collider_QuadSetNearestAC`: with `ATELEM_NEAREST`, only a hit nearer than this frame's
    /// nearest so far counts, and it undoes the AC side of the earlier one.
    fn quad_set_nearest_ac(&mut self, quad: &mut ColliderQuad, hit_pos: Vec3) -> bool {
        if quad.info.at_elem_flags & ATELEM_NEAREST == 0 {
            return true;
        }
        let dist = math3d::vec3f_dist_sq(v3s(quad.dim.dc_mid), hit_pos);
        if dist < quad.dim.ac_dist_sq {
            quad.dim.ac_dist_sq = dist;
            let base = quad.info.at_hit;
            let elem = quad.info.at_hit_elem.map(|h| h.elem);
            // Collider_ResetACBase(atHit), then Collider_ResetACElement(atHitElem).
            if let Some(r) = base {
                if r == self.rj {
                    self.reset_ac_of_j.get_or_insert(AcReset { base: false, elem: None }).base = true;
                } else if let Some(k) = self.set.index_of(r)
                    && let Some(c) = self.set.cols[k].as_mut()
                {
                    c.base_mut().reset_ac();
                }
            }
            if let Some(e) = elem {
                if e.col == self.rj {
                    self.reset_ac_of_j.get_or_insert(AcReset { base: false, elem: None }).elem = Some(e.elem);
                } else if let Some(k) = self.set.index_of(e.col)
                    && let Some(info) = self.set.cols[k].as_mut().and_then(|c| c.info_mut(e.elem as usize))
                {
                    info.reset_ac();
                }
            }
            return true;
        }
        false
    }

    /// `CollisionCheck_ATJntSphVsACJntSph`.
    fn jntsph_vs_jntsph(&mut self, at: &mut ColliderJntSph, ac: &mut ColliderJntSph) {
        for ai in 0..at.elements.len() {
            if skip_touch(&at.elements[ai].info) {
                continue;
            }
            for ci in 0..ac.elements.len() {
                if skip_bump(&ac.elements[ci].info) || no_shared_flags(&at.elements[ai].info, &ac.elements[ci].info) {
                    continue;
                }
                let (hit, _overlap, center_dist) = math3d::sph_vs_sph_overlap_center(&at.elements[ai].dim.world_sphere, &ac.elements[ci].dim.world_sphere);
                if hit {
                    let at_pos = v3s(at.elements[ai].dim.world_sphere.center);
                    let ac_pos = v3s(ac.elements[ci].dim.world_sphere.center);
                    let hit_pos = if !is_zero(center_dist) {
                        let k = ac.elements[ci].dim.world_sphere.radius as f32 / center_dist;
                        Vec3::new(((at_pos.x - ac_pos.x) * k) + ac_pos.x, ((at_pos.y - ac_pos.y) * k) + ac_pos.y, ((at_pos.z - ac_pos.z) * k) + ac_pos.z)
                    } else {
                        at_pos
                    };
                    let (ae, ce) = (&mut at.elements[ai].info, &mut ac.elements[ci].info);
                    self.set_at_vs_ac(&mut at.base, ae, ai as u16, &mut ac.base, ce, ci as u16, hit_pos);
                    // Without OC2_FIRST_ONLY the first hit ends the check; the flag (despite
                    // its decomp name) is what lets later elements be hit too.
                    if ac.base.oc_flags2 & OC2_FIRST_ONLY == 0 {
                        return;
                    }
                }
            }
        }
    }

    /// `CollisionCheck_ATJntSphVsACCyl`.
    fn jntsph_vs_cyl(&mut self, at: &mut ColliderJntSph, ac: &mut ColliderCylinder) {
        if at.elements.is_empty() || ac.dim.radius <= 0 || ac.dim.height <= 0 || skip_bump(&ac.info) {
            return;
        }
        for ai in 0..at.elements.len() {
            if skip_touch(&at.elements[ai].info) || no_shared_flags(&at.elements[ai].info, &ac.info) {
                continue;
            }
            let (hit, _overlap, center_dist) = math3d::sph_vs_cyl_overlap_center_dist(&at.elements[ai].dim.world_sphere, &ac.dim);
            if hit {
                let at_pos = v3s(at.elements[ai].dim.world_sphere.center);
                let ac_pos = v3s(ac.dim.pos);
                let hit_pos = hit_towards(at_pos, ac_pos, ac.dim.radius as f32, center_dist);
                let ae = &mut at.elements[ai].info;
                self.set_at_vs_ac(&mut at.base, ae, ai as u16, &mut ac.base, &mut ac.info, 0, hit_pos);
                return;
            }
        }
    }

    /// `CollisionCheck_ATCylVsACJntSph`.
    fn cyl_vs_jntsph(&mut self, at: &mut ColliderCylinder, ac: &mut ColliderJntSph) {
        if ac.elements.is_empty() || at.dim.radius <= 0 || at.dim.height <= 0 || skip_touch(&at.info) {
            return;
        }
        for ci in 0..ac.elements.len() {
            if skip_bump(&ac.elements[ci].info) || no_shared_flags(&at.info, &ac.elements[ci].info) {
                continue;
            }
            let (hit, _overlap, center_dist) = math3d::sph_vs_cyl_overlap_center_dist(&ac.elements[ci].dim.world_sphere, &at.dim);
            if hit {
                let at_pos = v3s(at.dim.pos);
                let ac_pos = v3s(ac.elements[ci].dim.world_sphere.center);
                let hit_pos = hit_towards(at_pos, ac_pos, ac.elements[ci].dim.world_sphere.radius as f32, center_dist);
                let ce = &mut ac.elements[ci].info;
                self.set_at_vs_ac(&mut at.base, &mut at.info, 0, &mut ac.base, ce, ci as u16, hit_pos);
                if ac.base.oc_flags2 & OC2_FIRST_ONLY == 0 {
                    break;
                }
            }
        }
    }

    /// `CollisionCheck_ATJntSphVsACTris`.
    fn jntsph_vs_tris(&mut self, at: &mut ColliderJntSph, ac: &mut ColliderTris) {
        for ai in 0..at.elements.len() {
            if skip_touch(&at.elements[ai].info) {
                continue;
            }
            for ci in 0..ac.elements.len() {
                if skip_bump(&ac.elements[ci].info) || no_shared_flags(&at.elements[ai].info, &ac.elements[ci].info) {
                    continue;
                }
                let (hit, hit_pos) = math3d::tri_vs_sph_intersect(&at.elements[ai].dim.world_sphere, &ac.elements[ci].dim);
                if hit {
                    let (ae, ce) = (&mut at.elements[ai].info, &mut ac.elements[ci].info);
                    self.set_at_vs_ac(&mut at.base, ae, ai as u16, &mut ac.base, ce, ci as u16, hit_pos);
                    return;
                }
            }
        }
    }

    /// `CollisionCheck_ATTrisVsACJntSph`.
    fn tris_vs_jntsph(&mut self, at: &mut ColliderTris, ac: &mut ColliderJntSph) {
        for ci in 0..ac.elements.len() {
            if skip_bump(&ac.elements[ci].info) {
                continue;
            }
            for ai in 0..at.elements.len() {
                if skip_touch(&at.elements[ai].info) || no_shared_flags(&at.elements[ai].info, &ac.elements[ci].info) {
                    continue;
                }
                let (hit, hit_pos) = math3d::tri_vs_sph_intersect(&ac.elements[ci].dim.world_sphere, &at.elements[ai].dim);
                if hit {
                    let (ae, ce) = (&mut at.elements[ai].info, &mut ac.elements[ci].info);
                    self.set_at_vs_ac(&mut at.base, ae, ai as u16, &mut ac.base, ce, ci as u16, hit_pos);
                    if ac.base.oc_flags2 & OC2_FIRST_ONLY == 0 {
                        return;
                    }
                }
            }
        }
    }

    /// `CollisionCheck_ATJntSphVsACQuad`.
    fn jntsph_vs_quad(&mut self, at: &mut ColliderJntSph, ac: &mut ColliderQuad) {
        if at.elements.is_empty() || skip_bump(&ac.info) {
            return;
        }
        let tris = quad_tris_ac(&ac.dim);
        for ai in 0..at.elements.len() {
            if skip_touch(&at.elements[ai].info) || no_shared_flags(&at.elements[ai].info, &ac.info) {
                continue;
            }
            let s = at.elements[ai].dim.world_sphere;
            let (h1, p1) = math3d::tri_vs_sph_intersect(&s, &tris[0]);
            let (hit, hit_pos) = if h1 { (true, p1) } else { math3d::tri_vs_sph_intersect(&s, &tris[1]) };
            if hit {
                let ae = &mut at.elements[ai].info;
                self.set_at_vs_ac(&mut at.base, ae, ai as u16, &mut ac.base, &mut ac.info, 0, hit_pos);
                return;
            }
        }
    }

    /// `CollisionCheck_ATQuadVsACJntSph`.
    fn quad_vs_jntsph(&mut self, at: &mut ColliderQuad, ac: &mut ColliderJntSph) {
        if ac.elements.is_empty() || skip_touch(&at.info) {
            return;
        }
        let tris = quad_tris_at(&at.dim);
        for ci in 0..ac.elements.len() {
            if skip_bump(&ac.elements[ci].info) || no_shared_flags(&at.info, &ac.elements[ci].info) {
                continue;
            }
            let s = ac.elements[ci].dim.world_sphere;
            let (h1, p1) = math3d::tri_vs_sph_intersect(&s, &tris[0]);
            let (hit, hit_pos) = if h1 { (true, p1) } else { math3d::tri_vs_sph_intersect(&s, &tris[1]) };
            if hit && self.quad_set_nearest_ac(at, hit_pos) {
                self.apply_deferred_reset_jntsph(ac);
                let ce = &mut ac.elements[ci].info;
                let (at_base, at_info) = (&mut at.base, &mut at.info);
                self.set_at_vs_ac(at_base, at_info, 0, &mut ac.base, ce, ci as u16, hit_pos);
                if ac.base.oc_flags2 & OC2_FIRST_ONLY == 0 {
                    return;
                }
            }
        }
    }

    /// `CollisionCheck_ATCylVsACCyl`.
    fn cyl_vs_cyl(&mut self, at: &mut ColliderCylinder, ac: &mut ColliderCylinder) {
        if at.dim.radius <= 0 || at.dim.height <= 0 || ac.dim.radius <= 0 || ac.dim.height <= 0 {
            return;
        }
        if skip_bump(&ac.info) || skip_touch(&at.info) || no_shared_flags(&at.info, &ac.info) {
            return;
        }
        let (hit, _dead_space, center_dist_xz) = math3d::cyl_outside_cyl_dist(&at.dim, &ac.dim);
        if hit {
            let hit_pos = if !is_zero(center_dist_xz) {
                let k = ac.dim.radius as f32 / center_dist_xz;
                Vec3::new(
                    (at.dim.pos[0] as f32 - ac.dim.pos[0] as f32) * k + ac.dim.pos[0] as f32,
                    ac.dim.pos[1] as f32 + ac.dim.y_shift as f32 + ac.dim.height as f32 * 0.5,
                    (at.dim.pos[2] as f32 - ac.dim.pos[2] as f32) * k + ac.dim.pos[2] as f32,
                )
            } else {
                v3s(ac.dim.pos)
            };
            self.set_at_vs_ac(&mut at.base, &mut at.info, 0, &mut ac.base, &mut ac.info, 0, hit_pos);
        }
    }

    /// `CollisionCheck_ATCylVsACTris`.
    fn cyl_vs_tris(&mut self, at: &mut ColliderCylinder, ac: &mut ColliderTris) {
        if at.dim.radius <= 0 || at.dim.height <= 0 || ac.elements.is_empty() || skip_touch(&at.info) {
            return;
        }
        for ci in 0..ac.elements.len() {
            if skip_bump(&ac.elements[ci].info) || no_shared_flags(&at.info, &ac.elements[ci].info) {
                continue;
            }
            let (hit, hit_pos) = math3d::cyl_tri_vs_intersect(&at.dim, &ac.elements[ci].dim);
            if hit {
                let ce = &mut ac.elements[ci].info;
                self.set_at_vs_ac(&mut at.base, &mut at.info, 0, &mut ac.base, ce, ci as u16, hit_pos);
                return;
            }
        }
    }

    /// `CollisionCheck_ATTrisVsACCyl`.
    fn tris_vs_cyl(&mut self, at: &mut ColliderTris, ac: &mut ColliderCylinder) {
        if ac.dim.radius <= 0 || ac.dim.height <= 0 || at.elements.is_empty() || skip_bump(&ac.info) {
            return;
        }
        for ai in 0..at.elements.len() {
            if skip_touch(&at.elements[ai].info) || no_shared_flags(&at.elements[ai].info, &ac.info) {
                continue;
            }
            let (hit, hit_pos) = math3d::cyl_tri_vs_intersect(&ac.dim, &at.elements[ai].dim);
            if hit {
                let ae = &mut at.elements[ai].info;
                self.set_at_vs_ac(&mut at.base, ae, ai as u16, &mut ac.base, &mut ac.info, 0, hit_pos);
                return;
            }
        }
    }

    /// `CollisionCheck_ATCylVsACQuad`.
    fn cyl_vs_quad(&mut self, at: &mut ColliderCylinder, ac: &mut ColliderQuad) {
        if at.dim.height <= 0 || at.dim.radius <= 0 {
            return;
        }
        if skip_touch(&at.info) || skip_bump(&ac.info) || no_shared_flags(&at.info, &ac.info) {
            return;
        }
        let tris = quad_tris_ac(&ac.dim);
        let (h1, p1) = math3d::cyl_tri_vs_intersect(&at.dim, &tris[0]);
        if h1 {
            self.set_at_vs_ac(&mut at.base, &mut at.info, 0, &mut ac.base, &mut ac.info, 0, p1);
        } else {
            let (h2, p2) = math3d::cyl_tri_vs_intersect(&at.dim, &tris[1]);
            if h2 {
                self.set_at_vs_ac(&mut at.base, &mut at.info, 0, &mut ac.base, &mut ac.info, 0, p2);
            }
        }
    }

    /// `CollisionCheck_ATQuadVsACCyl`: Player's sword against a cylinder (a bush, an NPC).
    fn quad_vs_cyl(&mut self, at: &mut ColliderQuad, ac: &mut ColliderCylinder) {
        if ac.dim.height <= 0 || ac.dim.radius <= 0 {
            return;
        }
        if skip_bump(&ac.info) || skip_touch(&at.info) || no_shared_flags(&at.info, &ac.info) {
            return;
        }
        let tris = quad_tris_at(&at.dim);
        let (h1, p1) = math3d::cyl_tri_vs_intersect(&ac.dim, &tris[0]);
        if h1 && self.quad_set_nearest_ac(at, p1) {
            self.apply_deferred_reset_cyl(ac);
            self.set_at_vs_ac(&mut at.base, &mut at.info, 0, &mut ac.base, &mut ac.info, 0, p1);
            return;
        }
        let (h2, p2) = math3d::cyl_tri_vs_intersect(&ac.dim, &tris[1]);
        if h2 && self.quad_set_nearest_ac(at, p2) {
            self.apply_deferred_reset_cyl(ac);
            self.set_at_vs_ac(&mut at.base, &mut at.info, 0, &mut ac.base, &mut ac.info, 0, p2);
        }
    }

    /// `CollisionCheck_ATTrisVsACTris`.
    fn tris_vs_tris(&mut self, at: &mut ColliderTris, ac: &mut ColliderTris) {
        for ci in 0..ac.elements.len() {
            if skip_bump(&ac.elements[ci].info) {
                continue;
            }
            for ai in 0..at.elements.len() {
                if skip_touch(&at.elements[ai].info) || no_shared_flags(&at.elements[ai].info, &ac.elements[ci].info) {
                    continue;
                }
                let (hit, hit_pos) = math3d::tri_vs_tri_intersect(&at.elements[ai].dim, &ac.elements[ci].dim);
                if hit {
                    let (ae, ce) = (&mut at.elements[ai].info, &mut ac.elements[ci].info);
                    self.set_at_vs_ac(&mut at.base, ae, ai as u16, &mut ac.base, ce, ci as u16, hit_pos);
                    return;
                }
            }
        }
    }

    /// `CollisionCheck_ATTrisVsACQuad`.
    fn tris_vs_quad(&mut self, at: &mut ColliderTris, ac: &mut ColliderQuad) {
        if at.elements.is_empty() || skip_bump(&ac.info) {
            return;
        }
        let tris = quad_tris_ac(&ac.dim);
        for ai in 0..at.elements.len() {
            if skip_touch(&at.elements[ai].info) || no_shared_flags(&at.elements[ai].info, &ac.info) {
                continue;
            }
            let (h1, p1) = math3d::tri_vs_tri_intersect(&tris[0], &at.elements[ai].dim);
            let (hit, hit_pos) = if h1 { (true, p1) } else { math3d::tri_vs_tri_intersect(&tris[1], &at.elements[ai].dim) };
            if hit {
                let ae = &mut at.elements[ai].info;
                self.set_at_vs_ac(&mut at.base, ae, ai as u16, &mut ac.base, &mut ac.info, 0, hit_pos);
                return;
            }
        }
    }

    /// `CollisionCheck_ATQuadVsACTris`.
    fn quad_vs_tris(&mut self, at: &mut ColliderQuad, ac: &mut ColliderTris) {
        if ac.elements.is_empty() || skip_touch(&at.info) {
            return;
        }
        // (2, 3, 1) and (1, 0, 2): the AC side's order, here on the AT quad.
        let tris = quad_tris_ac(&at.dim);
        for ci in 0..ac.elements.len() {
            if skip_bump(&ac.elements[ci].info) || no_shared_flags(&at.info, &ac.elements[ci].info) {
                continue;
            }
            let (h1, p1) = math3d::tri_vs_tri_intersect(&tris[0], &ac.elements[ci].dim);
            let (hit, hit_pos) = if h1 { (true, p1) } else { math3d::tri_vs_tri_intersect(&tris[1], &ac.elements[ci].dim) };
            if hit && self.quad_set_nearest_ac(at, hit_pos) {
                self.apply_deferred_reset_tris(ac);
                let ce = &mut ac.elements[ci].info;
                let (at_base, at_info) = (&mut at.base, &mut at.info);
                self.set_at_vs_ac(at_base, at_info, 0, &mut ac.base, ce, ci as u16, hit_pos);
                return;
            }
        }
    }

    /// `CollisionCheck_ATQuadVsACQuad`.
    fn quad_vs_quad(&mut self, at: &mut ColliderQuad, ac: &mut ColliderQuad) {
        if skip_touch(&at.info) || skip_bump(&ac.info) || no_shared_flags(&at.info, &ac.info) {
            return;
        }
        let at_tris = quad_tris_at(&at.dim);
        let ac_tris = quad_tris_at(&ac.dim);
        for i in 0..2 {
            for j in 0..2 {
                let (hit, hit_pos) = math3d::tri_vs_tri_intersect(&at_tris[j], &ac_tris[i]);
                if hit && self.quad_set_nearest_ac(at, hit_pos) {
                    self.apply_deferred_reset_quad(ac);
                    let (at_base, at_info) = (&mut at.base, &mut at.info);
                    self.set_at_vs_ac(at_base, at_info, 0, &mut ac.base, &mut ac.info, 0, hit_pos);
                    return;
                }
            }
        }
    }

    // A reset `Collider_QuadSetNearestAC` made on this pair's own AC collider applies before
    // the new hit is recorded on it.
    fn apply_deferred_reset_cyl(&mut self, ac: &mut ColliderCylinder) {
        if let Some(r) = self.reset_ac_of_j.take() {
            if r.base {
                ac.base.reset_ac();
            }
            if r.elem == Some(0) {
                ac.info.reset_ac();
            }
        }
    }
    fn apply_deferred_reset_quad(&mut self, ac: &mut ColliderQuad) {
        if let Some(r) = self.reset_ac_of_j.take() {
            if r.base {
                ac.base.reset_ac();
            }
            if r.elem == Some(0) {
                ac.info.reset_ac();
            }
        }
    }
    fn apply_deferred_reset_jntsph(&mut self, ac: &mut ColliderJntSph) {
        if let Some(r) = self.reset_ac_of_j.take() {
            if r.base {
                ac.base.reset_ac();
            }
            if let Some(e) = r.elem.and_then(|e| ac.elements.get_mut(e as usize)) {
                e.info.reset_ac();
            }
        }
    }
    fn apply_deferred_reset_tris(&mut self, ac: &mut ColliderTris) {
        if let Some(r) = self.reset_ac_of_j.take() {
            if r.base {
                ac.base.reset_ac();
            }
            if let Some(e) = r.elem.and_then(|e| ac.elements.get_mut(e as usize)) {
                e.info.reset_ac();
            }
        }
    }
}

/// The hit point on the line from `ac_pos` to `at_pos`, `radius` out from `ac_pos`, or `at_pos`
/// when that's past it (`CollisionCheck_ATJntSphVsACCyl` and `_CylVsJntSph`).
fn hit_towards(at_pos: Vec3, ac_pos: Vec3, radius: f32, center_dist: f32) -> Vec3 {
    if !is_zero(center_dist) {
        let k = radius / center_dist;
        if k <= 1.0 {
            return Vec3::new(((at_pos.x - ac_pos.x) * k) + ac_pos.x, ((at_pos.y - ac_pos.y) * k) + ac_pos.y, ((at_pos.z - ac_pos.z) * k) + ac_pos.z);
        }
    }
    at_pos
}

/// `CollisionCheck_Incompatible`.
fn incompatible(left: &ColliderBase, right: &ColliderBase) -> bool {
    if left.oc_flags1 & right.oc_flags2 & OC1_TYPE_ALL == 0
        || left.oc_flags2 & right.oc_flags1 & OC1_TYPE_ALL == 0
        || (left.oc_flags2 & OC2_UNK1 != 0 && right.oc_flags2 & OC2_UNK2 != 0)
        || (right.oc_flags2 & OC2_UNK1 != 0 && left.oc_flags2 & OC2_UNK2 != 0)
    {
        return true;
    }
    left.actor == right.actor
}

/// `MASSTYPE_*`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MassType {
    Immovable,
    Heavy,
    Normal,
}

/// `CollisionCheck_GetMassType`.
fn mass_type(mass: u8) -> MassType {
    match mass {
        MASS_IMMOVABLE => MassType::Immovable,
        MASS_HEAVY => MassType::Heavy,
        _ => MassType::Normal,
    }
}

/// `CollisionCheck_SetOCvsOC`: marks both sides and pushes the actors apart by `overlap`.
#[allow(clippy::too_many_arguments)]
fn set_oc_vs_oc(left: &mut ColliderBase, left_info: &mut ColliderElement, left_pos: Vec3, right: &mut ColliderBase, right_info: &mut ColliderElement, right_pos: Vec3, overlap: f32, actors: &mut ActorContext) {
    let (left_actor, right_actor) = (left.actor, right.actor);
    left.oc_flags1 |= OC1_HIT;
    left.oc = right_actor;
    left_info.oc_elem_flags |= OCELEM_HIT;
    if right.oc_flags2 & OC2_TYPE_PLAYER != 0 {
        left.oc_flags2 |= OC2_HIT_PLAYER;
    }
    right.oc = left_actor;
    right.oc_flags1 |= OC1_HIT;
    right_info.oc_elem_flags |= OCELEM_HIT;
    if left.oc_flags2 & OC2_TYPE_PLAYER != 0 {
        right.oc_flags2 |= OC2_HIT_PLAYER;
    }
    let (Some(lh), Some(rh)) = (left_actor, right_actor) else { return };
    if left.oc_flags1 & OC1_NO_PUSH != 0 || right.oc_flags1 & OC1_NO_PUSH != 0 {
        return;
    }
    let (Some(lm), Some(rm)) = (actors.actor(lh).map(|a| a.col_chk_info.mass), actors.actor(rh).map(|a| a.col_chk_info.mass)) else { return };
    // The decomp's names are swapped (`rightMassType` is the left actor's); the logic below
    // is consistent with that.
    let right_mass_type = mass_type(lm);
    let left_mass_type = mass_type(rm);
    let (mut left_mass, mut right_mass) = (lm as f32, rm as f32);
    let mut total_mass = left_mass + right_mass;
    if is_zero(total_mass) {
        left_mass = 1.0;
        right_mass = 1.0;
        total_mass = 2.0;
    }
    let mut x_delta = right_pos.x - left_pos.x;
    let mut z_delta = right_pos.z - left_pos.z;
    let xz_dist = (x_delta * x_delta + z_delta * z_delta).sqrt();
    let (left_ratio, right_ratio) = match (right_mass_type, left_mass_type) {
        (MassType::Immovable, MassType::Immovable) => return,
        (MassType::Immovable, _) => (0.0, 1.0),
        (MassType::Heavy, MassType::Immovable) => (1.0, 0.0),
        (MassType::Heavy, MassType::Heavy) => (0.5, 0.5),
        (MassType::Heavy, MassType::Normal) => (0.0, 1.0),
        (MassType::Normal, MassType::Normal) => {
            let inv = 1.0 / total_mass;
            (right_mass * inv, left_mass * inv)
        }
        (MassType::Normal, _) => (1.0, 0.0),
    };
    let mut push = |h: ActorHandle, dx: f32, dz: f32| {
        if let Some(a) = actors.actor_mut(h) {
            a.col_chk_info.displacement.x += dx;
            a.col_chk_info.displacement.z += dz;
        }
    };
    if !is_zero(xz_dist) {
        x_delta *= overlap / xz_dist;
        z_delta *= overlap / xz_dist;
        push(lh, -x_delta * left_ratio, -z_delta * left_ratio);
        push(rh, x_delta * right_ratio, z_delta * right_ratio);
    } else if overlap != 0.0 {
        push(lh, -overlap * left_ratio, 0.0);
        push(rh, overlap * right_ratio, 0.0);
    } else {
        push(lh, -left_ratio, 0.0);
        push(rh, right_ratio, 0.0);
    }
}

/// `CollisionCheck_OC_JntSphVsJntSph`.
fn oc_jntsph_vs_jntsph(left: &mut ColliderJntSph, _rl: ColliderRef, right: &mut ColliderJntSph, _rr: ColliderRef, actors: &mut ActorContext) {
    for li in 0..left.elements.len() {
        if left.elements[li].info.oc_elem_flags & OCELEM_ON == 0 {
            continue;
        }
        for ri in 0..right.elements.len() {
            if right.elements[ri].info.oc_elem_flags & OCELEM_ON == 0 {
                continue;
            }
            let (hit, overlap) = math3d::sph_vs_sph_overlap(&left.elements[li].dim.world_sphere, &right.elements[ri].dim.world_sphere);
            if hit {
                let lp = v3s(left.elements[li].dim.world_sphere.center);
                let rp = v3s(right.elements[ri].dim.world_sphere.center);
                let (le, re) = (&mut left.elements[li].info, &mut right.elements[ri].info);
                set_oc_vs_oc(&mut left.base, le, lp, &mut right.base, re, rp, overlap, actors);
            }
        }
    }
}

/// `CollisionCheck_OC_JntSphVsCyl`.
fn oc_jntsph_vs_cyl(left: &mut ColliderJntSph, _rl: ColliderRef, right: &mut ColliderCylinder, _rr: ColliderRef, actors: &mut ActorContext) {
    if left.elements.is_empty() || right.base.oc_flags1 & OC1_ON == 0 || right.info.oc_elem_flags & OCELEM_ON == 0 {
        return;
    }
    for li in 0..left.elements.len() {
        if left.elements[li].info.oc_elem_flags & OCELEM_ON == 0 {
            continue;
        }
        let (hit, overlap) = math3d::sph_vs_cyl_overlap_dist(&left.elements[li].dim.world_sphere, &right.dim);
        if hit {
            let lp = v3s(left.elements[li].dim.world_sphere.center);
            let rp = v3s(right.dim.pos);
            let le = &mut left.elements[li].info;
            set_oc_vs_oc(&mut left.base, le, lp, &mut right.base, &mut right.info, rp, overlap, actors);
        }
    }
}

/// `CollisionCheck_OC_CylVsCyl`.
fn oc_cyl_vs_cyl(left: &mut ColliderCylinder, _rl: ColliderRef, right: &mut ColliderCylinder, _rr: ColliderRef, actors: &mut ActorContext) {
    if left.base.oc_flags1 & OC1_ON == 0 || right.base.oc_flags1 & OC1_ON == 0 {
        return;
    }
    if left.info.oc_elem_flags & OCELEM_ON == 0 || right.info.oc_elem_flags & OCELEM_ON == 0 {
        return;
    }
    let (hit, dead_space) = math3d::cyl_outside_cyl(&left.dim, &right.dim);
    if hit {
        let (lp, rp) = (v3s(left.dim.pos), v3s(right.dim.pos));
        set_oc_vs_oc(&mut left.base, &mut left.info, lp, &mut right.base, &mut right.info, rp, dead_space, actors);
    }
}
