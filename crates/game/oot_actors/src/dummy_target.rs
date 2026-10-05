//! The sandbox's dummy Z-target (the training dummy): not a game actor, a stand-in enemy to
//! lock on to and to hit. It's targetable and hostile (`ACTOR_FLAG_ATTENTION_ENABLED |
//! ACTOR_FLAG_HOSTILE`), `targetMode` 3 (a 350 range), with its focus 40 above its feet, and it
//! doesn't move. It draws as a box.
//!
//! Its body is a cylinder 15 wide and 60 high:
//! - **AC:** Player's attacks hit it and take their damage from a damage table, the Deku
//!   Baba's (`En_Dekubaba`'s `sDamageTableNormal`), through `CollisionCheck_ApplyDamage` and
//!   `Actor_ApplyDamage`. It flashes red (`Actor_SetColorFilter`), keeps count of the hits
//!   and never dies: at no health it's back to full.
//! - **AT and OC** (a hurting dummy, `DummyTarget::hurting`): an immovable body, and touching it
//!   hurts Link, half a heart, with
//!   a hit special effect (`HIT_SPECIAL_EFFECT_*`: fire sets him burning, ice freezes him, electric
//!   shocks him, knockback knocks him down).

use eng_collision::math3d::Cylinder16;
use eng_gfx::{DrawCmd, MeshKey};
use glam::{Mat4, Vec3};
use oot_game::actor::{ACTOR_FLAG_ATTENTION_ENABLED, ACTOR_FLAG_HOSTILE, Actor, COLORFILTER_BUFFLAG_OPA, COLORFILTER_COLORFLAG_RED};
use oot_game::actor_ctx::{ACTOR_SANDBOX_DUMMY_TARGET, ACTORCAT_ENEMY, ActorImpl, ActorProfile};
use oot_game::collision_check::{self as cc, ColliderCylinder, ColliderCylinderInit, ColliderElementDamageInfoACInit, ColliderElementDamageInfoAT, ColliderElementInit, ColliderInit, ColliderMut};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};

pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_SANDBOX_DUMMY_TARGET,
    name: "Sandbox_Dummy_Target",
    category: ACTORCAT_ENEMY,
    flags: ACTOR_FLAG_ATTENTION_ENABLED | ACTOR_FLAG_HOSTILE,
    object: "gameplay_keep",
};

/// The health it starts with and goes back to.
const HEALTH: u8 = 8;
/// What its touch takes from Link: half a heart (the Deku Baba's bite's `atDmgInfo.damage`).
const TOUCH_DAMAGE: u8 = 0x08;

fn cylinder_init(hurts: Option<u8>) -> ColliderCylinderInit {
    ColliderCylinderInit {
        base: ColliderInit {
            col_type: cc::COL_MATERIAL_HIT0,
            at_flags: if hurts.is_some() { cc::AT_ON | cc::AT_TYPE_ENEMY } else { cc::AT_NONE },
            ac_flags: cc::AC_ON | cc::AC_TYPE_PLAYER,
            oc_flags1: if hurts.is_some() { cc::OC1_ON | cc::OC1_TYPE_ALL } else { cc::OC1_NONE },
            oc_flags2: cc::OC2_TYPE_1,
            shape: cc::COLSHAPE_CYLINDER,
        },
        info: ColliderElementInit {
            elem_material: cc::ELEM_MATERIAL_UNK0,
            at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0xFFCF_FFFF, hit_special_effect: hurts.unwrap_or(cc::HIT_SPECIAL_EFFECT_NONE), damage: TOUCH_DAMAGE },
            ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: cc::HIT_BACKLASH_NONE, defense: 0 },
            at_elem_flags: if hurts.is_some() { cc::ATELEM_ON | cc::ATELEM_SFX_NORMAL } else { cc::ATELEM_NONE },
            ac_elem_flags: cc::ACELEM_ON,
            oc_elem_flags: if hurts.is_some() { cc::OCELEM_ON } else { cc::OCELEM_NONE },
        },
        dim: Cylinder16 { radius: 15, height: 60, y_shift: 0, pos: [0; 3] },
    }
}

#[derive(Debug, Clone)]
pub struct DummyTarget {
    pub actor: Actor,
    /// The body (AC, and AT when it hurts).
    pub collider: ColliderCylinder,
    /// What its touch does to Link (`HIT_SPECIAL_EFFECT_*`), when it hurts.
    pub hurts: Option<u8>,
    /// The hits it has taken, and the last one's `colChkInfo.damage` and `damageReaction`.
    pub hits: u32,
    pub last_damage: u8,
    pub last_reaction: u8,
}

impl DummyTarget {
    /// A dummy enemy standing at `pos`.
    pub fn new(pos: Vec3) -> DummyTarget {
        Self::build(pos, None)
    }

    /// A dummy whose touch hurts Link, with `effect` (`HIT_SPECIAL_EFFECT_*`).
    pub fn hurting(pos: Vec3, effect: u8) -> DummyTarget {
        Self::build(pos, Some(effect))
    }

    fn build(pos: Vec3, hurts: Option<u8>) -> DummyTarget {
        let mut actor = Actor::new(pos, 0);
        PROFILE.apply(&mut actor);
        actor.focus_pos = pos + Vec3::Y * 40.0;
        actor.target_mode = 3;
        actor.col_chk_info.damage_table = Some(&crate::en_dekubaba::S_DAMAGE_TABLE_NORMAL);
        actor.col_chk_info.health = HEALTH;
        actor.col_chk_info.mass = cc::MASS_IMMOVABLE;
        DummyTarget { actor, collider: ColliderCylinder::new(&cylinder_init(hurts)), hurts, hits: 0, last_damage: 0, last_reaction: 0 }
    }
}

impl ActorImpl for DummyTarget {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    fn update(&mut self, play: &mut PlayState) {
        if self.collider.base.ac_flags & cc::AC_HIT != 0 {
            self.collider.base.ac_flags &= !cc::AC_HIT;
            self.hits += 1;
            self.last_damage = self.actor.col_chk_info.damage;
            self.last_reaction = self.actor.col_chk_info.damage_reaction;
            if self.actor.apply_damage() == 0 {
                self.actor.col_chk_info.health = HEALTH;
            }
            self.actor.set_color_filter(COLORFILTER_COLORFLAG_RED, 255, COLORFILTER_BUFFLAG_OPA, 8);
        }
        if self.collider.base.at_flags & cc::AT_HIT != 0 {
            self.collider.base.at_flags &= !cc::AT_HIT;
        }
        self.collider.update(&self.actor);
        play.collision_check_set_ac(&self.actor, 0, &mut self.collider);
        if self.hurts.is_some() {
            play.collision_check_set_at(&self.actor, 0, &mut self.collider);
            play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
        }
    }
    fn draw(&self, st: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        out.opa.push(DrawCmd::new(MeshKey::named(oot_game::play::builtin::TARGET_BOX), Mat4::from_translation(st.pos)));
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
