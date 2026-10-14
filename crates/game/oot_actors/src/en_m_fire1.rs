//! `En_M_Fire1` (`ovl_En_M_Fire1/z_en_m_fire1.c`): a Deku Nut's stun, spawned where the nut
//! burst (`EnArrow_Fly`): a cylinder 200 wide and 200 tall from there up attacking with
//! `DMG_DEKU_NUT` (no damage of its own) while its timer steps from 0 to 1 by 0.2: four frames
//! of attack, gone on the fifth. Negative params make it an item action (`ACTORCAT_ITEMACTION`)
//! rather than a misc actor.
//!
//! The whole overlay is ported. It has no draw.
//!
//! Params (none placed: `EnArrow_Fly` spawns it with 0): `params < 0` bit 15 (1: an item action);
//! bits 0 to 14 read by nothing.
//! Flags: none.

use eng_collision::math3d::Cylinder16;
use oot_game::actor::Actor;
use oot_game::actor_ctx::{ACTORCAT_ITEMACTION, ACTORCAT_MISC, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, Field, Kind};
use oot_game::collision_check::*;
use oot_game::play::PlayState;

/// `ACTOR_EN_M_FIRE1` (`actor_table.h`).
pub const ACTOR_EN_M_FIRE1: i16 = 0x0056;

/// `En_M_Fire1_Profile`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_M_FIRE1, name: "En_M_Fire1", category: ACTORCAT_MISC, flags: 0, object: "gameplay_keep", schema: &SCHEMA };

/// `this->actor.params < 0`: the sign bit.
pub const ITEM_ACTION: Field = Field::params("params < 0", 15, 1, Kind::Named(&[(0, "a misc actor"), (1, "an item action (ACTORCAT_ITEMACTION)")]), "its actor category");

/// En_M_Fire1's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema { fields: &[ITEM_ACTION], flags: &[], todo: false };

/// `sCylinderInit`.
pub const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_ON | AT_TYPE_PLAYER, ac_flags: AC_NONE, oc_flags1: OC1_NONE, oc_flags2: OC2_TYPE_PLAYER, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK2,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0x0000_0001, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x00 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: HIT_BACKLASH_NONE, defense: 0x00 },
        at_elem_flags: ATELEM_ON | ATELEM_SFX_NONE,
        ac_elem_flags: ACELEM_NONE,
        oc_elem_flags: OCELEM_NONE,
    },
    dim: Cylinder16 { radius: 200, height: 200, y_shift: 0, pos: [0; 3] },
};

pub struct EnMFire1 {
    pub actor: Actor,
    pub collider: ColliderCylinder,
    pub timer: f32,
}

impl EnMFire1 {
    /// `EnMFire1_Init`.
    pub fn init(mut actor: Actor, _play: &mut PlayState) -> Box<dyn ActorImpl> {
        if ITEM_ACTION.get(actor.params) != 0 {
            // Actor_ChangeCategory(.., ACTORCAT_ITEMACTION) (Actor_Spawn moves it to the list).
            actor.category = ACTORCAT_ITEMACTION;
        }
        let collider = ColliderCylinder::new(&CYLINDER_INIT);
        Box::new(EnMFire1 { actor, collider, timer: 0.0 })
    }
}

impl ActorImpl for EnMFire1 {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `EnMFire1_Update`: the timer to 1 by 0.2; there, gone; until then the cylinder at it and
    /// attacking (`CollisionCheck_SetAT`).
    fn update(&mut self, play: &mut PlayState) {
        if eng_math::step_to_f(&mut self.timer, 1.0, 0.2) {
            self.actor.kill();
        } else {
            self.collider.update(&self.actor);
            play.collision_check_set_at(&self.actor, 0, &mut self.collider);
        }
    }
    /// `EnMFire1_Destroy`: `Collider_DestroyCylinder` (nothing to free).
    fn destroy(&mut self, _play: &mut PlayState) {}
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
