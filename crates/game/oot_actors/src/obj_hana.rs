//! `Obj_Hana` (`ovl_Obj_Hana/z_obj_hana.c`): a flower, a small rock or a bush from
//! `gameplay_field_keep`, by `params & 3`. The rock and the bush have a body that Link bumps
//! into (OC, `MASS_IMMOVABLE`). The bush is gone once Zelda's letter is had.
//!
//! Params: `type` bits 0 to 1 (`sHanaParams`: 0 a flower, 1 a small rock, 2 a bush).
//! Flags: reads EVENTCHKINF_OBTAINED_ZELDAS_LETTER (type 2: the bush is gone).

use eng_collision::math3d::Cylinder16;
use oot_game::actor::Actor;
use oot_game::actor_ctx::{ACTORCAT_PROP, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, Field, FlagUse, Kind};
use oot_game::event_chk_inf;
use oot_game::collision_check::*;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::save::EVENTCHKINF_OBTAINED_ZELDAS_LETTER;

pub const ACTOR_OBJ_HANA: i16 = 0x014F;

/// `Obj_Hana_Profile`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_OBJ_HANA, name: "Obj_Hana", category: ACTORCAT_PROP, flags: 0, object: "gameplay_field_keep", schema: &SCHEMA };

/// `PARAMS_GET_U(this->actor.params, 0, 2)`: which, an index into `sHanaParams`.
pub const TYPE: Field = Field::params("type", 0, 2, Kind::Named(&[(0, "a flower"), (1, "a small rock"), (2, "a bush")]), "a flower, a small rock or a bush");

/// Obj_Hana's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[TYPE],
    flags: &[FlagUse::new(event_chk_inf!(EVENTCHKINF_OBTAINED_ZELDAS_LETTER)).reads().note("type 2: the bush is gone")],
    todo: false,
};

/// `sCylinderInit`.
const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_NONE, ac_flags: AC_NONE, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_2, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: 0, damage: 0 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0, hit_backlash: 0, defense: 0 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_NONE,
        oc_elem_flags: OCELEM_ON,
    },
    dim: Cylinder16 { radius: 8, height: 10, y_shift: 0, pos: [0; 3] },
};

/// `sColChkInfoInit`.
const COL_CHK_INFO_INIT: CollisionCheckInfoInit = CollisionCheckInfoInit { health: 0, cyl_radius: 12, cyl_height: 60, mass: MASS_IMMOVABLE };

/// `HanaParams`.
struct HanaParams {
    dlist: &'static str,
    scale: f32,
    y_offset: f32,
    radius: i16,
    height: i16,
}

/// `sHanaParams`: a flower (`gHanaDL`, no body), a small rock (`gFieldKakeraDL`), a bush
/// (`gFieldBushDL`).
const HANA_PARAMS: [HanaParams; 3] = [
    HanaParams { dlist: "gHanaDL", scale: 0.01, y_offset: 0.0, radius: -1, height: 0 },
    HanaParams { dlist: "gFieldKakeraDL", scale: 0.1, y_offset: 58.0, radius: 10, height: 18 },
    HanaParams { dlist: "gFieldBushDL", scale: 0.4, y_offset: 0.0, radius: 12, height: 44 },
];

pub struct ObjHana {
    pub actor: Actor,
    pub collider: ColliderCylinder,
}

impl ObjHana {
    fn params(&self) -> &'static HanaParams {
        // `params & 3`: 3 would read past the table.
        &HANA_PARAMS[TYPE.get(self.actor.params).min(2) as usize]
    }

    /// `ObjHana_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let ty = TYPE.get(actor.params).min(2) as usize;
        let p = &HANA_PARAMS[ty];
        // sInitChain: scale 0.01 (overwritten); cullingVolumeDistance 900, cullingVolumeScale 60,
        // cullingVolumeDownward 800.
        actor.culling_volume_distance = 900.0;
        actor.culling_volume_scale = 60.0;
        actor.culling_volume_downward = 800.0;
        actor.scale = glam::Vec3::splat(p.scale);
        actor.shape_y_offset = p.y_offset;
        let mut collider = ColliderCylinder::default();
        if p.radius >= 0 {
            collider = ColliderCylinder::new(&CYLINDER_INIT);
            collider.update(&actor);
            collider.dim.radius = p.radius;
            collider.dim.height = p.height;
            actor.col_chk_info.set_info(None, &COL_CHK_INFO_INIT);
        }
        if ty == 2 && play.save.get_event_chk_inf(EVENTCHKINF_OBTAINED_ZELDAS_LETTER) {
            actor.kill();
        }
        Box::new(ObjHana { actor, collider })
    }
}

impl ActorImpl for ObjHana {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `ObjHana_Update`.
    fn update(&mut self, play: &mut PlayState) {
        if self.params().radius >= 0 && self.actor.xz_dist_to_player < 400.0 {
            play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
        }
    }
    /// `ObjHana_Draw`: `Gfx_DrawDListOpa`.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        crate::gfx_draw_dlist_opa(out, "gameplay_field_keep", self.params().dlist, rs);
    }
    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        (id == 0 && self.params().radius >= 0).then_some(ColliderMut::Cylinder(&mut self.collider))
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
