//! `En_It` (`ovl_En_It/z_en_it.c`): one of the graveyard's dig spots for Dampé's
//! Heart-Pounding Gravedigging Tour, placed on its child's night layer (eight of them).
//!
//! Invisible (no draw): an OC cylinder 40 across and 10 high (`OC1_ON | OC1_NO_PUSH`, with no
//! `OC1_TYPE_*` bit, so it touches nothing), set in the OC list every frame. Dampé
//! (`En_Tk`, `EnTk_CheckNextSpot`) finds the spots in the prop list by their id, within 40 of him
//! across and 10 above his floor, and digs up a prize on one; the collider isn't what he reads.
//!
//! Ported whole.
//!
//! Params: none read (placed as 0; `EnIt_Init` sets them to 0x0D05, which `EnTk_CheckNextSpot`
//! returns, 0 or more meaning "a spot").
//! Flags: none.

use eng_collision::math3d::Cylinder16;
use oot_game::actor::Actor;
use oot_game::actor_ctx::{ACTORCAT_PROP, ActorImpl, ActorProfile};
use oot_game::collision_check::*;
use oot_game::play::PlayState;

/// `ACTOR_EN_IT` (`actor_table.h`: 0x0196).
pub const ACTOR_EN_IT: i16 = 0x0196;

/// `En_It_Profile`: `ACTORCAT_PROP`, no flags, `OBJECT_GAMEPLAY_KEEP` (`object_table.h`: 0x0001).
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_IT, name: "En_It", category: ACTORCAT_PROP, flags: 0, object: "gameplay_keep" };

/// The params `EnIt_Init` sets.
pub const EN_IT_PARAMS: i16 = 0x0D05;

/// `sCylinderInit`.
const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_NONE, ac_flags: AC_NONE, oc_flags1: OC1_ON | OC1_NO_PUSH, oc_flags2: OC2_TYPE_1, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0x0000_0000, hit_special_effect: 0, damage: 0 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0x0000_0000, hit_backlash: 0, defense: 0 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_NONE,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 40, height: 10, y_shift: 0, pos: [0; 3] },
};

/// `sColChkInfoInit`.
const COL_CHK_INFO_INIT: CollisionCheckInfoInit2 = CollisionCheckInfoInit2 { health: 0, cyl_radius: 0, cyl_height: 0, cyl_y_shift: 0, mass: MASS_IMMOVABLE };

pub struct EnIt {
    pub actor: Actor,
    pub collider: ColliderCylinder,
}

impl EnIt {
    /// `EnIt_Init`.
    pub fn init(actor: Actor, _play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut this = EnIt { actor, collider: ColliderCylinder::new(&CYLINDER_INIT) };
        this.actor.params = EN_IT_PARAMS;
        this.actor.col_chk_info.set_info2(None, &COL_CHK_INFO_INIT);
        Box::new(this)
    }
}

impl ActorImpl for EnIt {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `EnIt_Update`.
    fn update(&mut self, play: &mut PlayState) {
        self.collider.update(&self.actor);
        play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
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
