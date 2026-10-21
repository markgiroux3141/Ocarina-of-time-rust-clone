//! `Bg_Spot08_Bakudankabe` (`ovl_Bg_Spot08_Bakudankabe/z_bg_spot08_bakudankabe.c`): Zora's
//! Fountain's bombable wall (spot08 room 0: 0x0003 at (316, 60, 2634), facing +z, in front of
//! exit 5, the way to Farore's Wind's Great Fairy). A DynaPoly actor of `object_spot08_obj`
//! (`gZorasFountainBombableWallCol`, registered before `sInitChain` gives it scale 1), drawn
//! `gZorasFountainBombableWallDL`.
//!
//! Its collider is three spheres (`sJntSphElementsInit`: 70 across at (0, 50, 50), (-100, 50, 50)
//! and (100, 50, 50) in its model, all on limb 0) that only `DMG_EXPLOSIVE` (`0x00000008`) hits;
//! set for hits while Link is within 800 across, placed by the draw (`Collider_UpdateSpheres` for
//! limbs 0, 1 and 2: only limb 0's matter, the three are all limb 0's). Hit (`func_808B0324`):
//! 24 pieces of rock (`EffectSsKakera`, `gFieldKakeraDL`) up its face, 440 across and 20 deep at
//! random, each `i × 65 / 12` higher from 20 up, flying up at -2.4 to 9.6; 10 to 84 big (under 25
//! gravity -300, under 50 -360, else -420), tumbling fast (65) four times in ten, else slower
//! (33); then dust at three points of it (`D_808B08AC`, `func_80033480`: 120 across, four,
//! 120 big growing 160, lit). Its switch flag set, `NA_SE_EV_WALL_BROKEN` (40 frames at its
//! position), the chime (`NA_SE_SY_CORRECT_CHIME`), and it's gone. With its flag set at init it's
//! gone at once.
//!
//! The whole overlay is ported.
//!
//! Params: `switchFlag` bits 0 to 5 (`PARAMS_GET_U(params, 0, 6)`: set when it's blown up; set at
//! init, it's gone); bits 6 and 7 unused (layer 4's 0x00FF sets them).
//! Flags: reads and sets switch flag `switchFlag`.

use std::sync::Arc;

use eng_collision::collision::CollisionHeader;
use eng_collision::dyna::{BG_ACTOR_MAX, BgActorSource};
use eng_collision::math3d::Sphere16;
use eng_math::{cos_s, sin_s};
use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_IGNORE_POINT_LIGHTS, Actor};
use oot_game::actor_ctx::{ACTORCAT_BG, ActorImpl, ActorProfile, func_80033480};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind};
use oot_game::audio::sfx::{NA_SE_EV_WALL_BROKEN, NA_SE_SY_CORRECT_CHIME};
use oot_game::collision_check::*;
use oot_game::effect::kakera::{KAKERA_COLOR_NONE, OBJECT_GAMEPLAY_FIELD_KEEP};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};

/// `ACTOR_BG_SPOT08_BAKUDANKABE` (`actor_table.h`: 0x01A8).
pub const ACTOR_BG_SPOT08_BAKUDANKABE: i16 = 0x01A8;
/// `OBJECT_SPOT08_OBJ` (`object_table.h`: 0x0074).
pub const OBJECT: &str = "object_spot08_obj";
pub const COLLISION: &str = "gZorasFountainBombableWallCol";
pub const DL: &str = "gZorasFountainBombableWallDL";

/// `Bg_Spot08_Bakudankabe_Profile`: `ACTORCAT_BG`, `ACTOR_FLAG_IGNORE_POINT_LIGHTS`,
/// `OBJECT_SPOT08_OBJ`.
pub const PROFILE: ActorProfile =
    ActorProfile { id: ACTOR_BG_SPOT08_BAKUDANKABE, name: "Bg_Spot08_Bakudankabe", category: ACTORCAT_BG, flags: ACTOR_FLAG_IGNORE_POINT_LIGHTS, object: OBJECT, schema: &SCHEMA };

/// `PARAMS_GET_U(params, 0, 6)`: the switch flag.
pub const SWITCH_FLAG: Field = Field::params("switchFlag", 0, 6, Kind::SwitchFlag, "set when it's blown up; set at init, it's gone");

/// Bits 6 and 7: the Sapphire cutscene's layer (4) places it as 0x00FF (flag 0x3F), in a room
/// without `object_spot08_obj` (never spawned).
pub const SWITCH_FLAG_HIGH: Field = Field::params("switchFlag bits 6 and 7", 6, 2, Kind::Unused, "set with a switch flag of 0x3F (0xFF) in the data; read by nothing");

/// Bg_Spot08_Bakudankabe's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[SWITCH_FLAG, SWITCH_FLAG_HIGH],
    flags: &[FlagUse::new(Flag::Switch("switchFlag")).reads().sets().note("set as it's blown up; set at init, it's gone")],
    todo: false,
    objects: &[],
    texts: &[],
};

/// One of `sJntSphElementsInit`'s spheres: limb 0, 70 across at `center`, hit by `0x00000008`
/// (`DMG_EXPLOSIVE`), scale 100.
const fn sphere(center: [i16; 3]) -> ColliderJntSphElementInit {
    ColliderJntSphElementInit {
        info: ColliderElementInit {
            elem_material: ELEM_MATERIAL_UNK0,
            at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0x0000_0000, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x00 },
            ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0x0000_0008, hit_backlash: HIT_BACKLASH_NONE, defense: 0x00 },
            at_elem_flags: ATELEM_NONE,
            ac_elem_flags: ACELEM_ON,
            oc_elem_flags: OCELEM_NONE,
        },
        limb: 0,
        model_sphere: Sphere16 { center, radius: 70 },
        scale: 100,
    }
}

/// `sJntSphElementsInit`.
pub const JNT_SPH_ELEMENTS: [ColliderJntSphElementInit; 3] = [sphere([0, 50, 50]), sphere([-100, 50, 50]), sphere([100, 50, 50])];

/// `sJntSphInit`: hit by Player's side (`AC_TYPE_PLAYER`: his explosives), no pushing.
pub const JNT_SPH_INIT: ColliderInit = ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_NONE, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_NONE, oc_flags2: OC2_TYPE_2, shape: COLSHAPE_JNTSPH };

/// `D_808B08AC`: the dust's three points (x across, y up, z out).
pub const D_808B08AC: [Vec3; 3] = [Vec3::new(0.0, 116.65, 50.0), Vec3::new(115.0, 95.0, 10.0), Vec3::new(-115.0, 95.0, 10.0)];

pub struct BgSpot08Bakudankabe {
    /// `dyna.actor`, `dyna.bgId`.
    pub actor: Actor,
    pub bg: u16,
    pub collider: ColliderJntSph,
}

fn source(a: &Actor) -> BgActorSource {
    let r = a.shape_rot;
    BgActorSource { pos: a.world_pos, shape_rot: [r.x, r.y, r.z], scale: a.scale, shape_y_offset: a.shape_y_offset }
}

impl BgSpot08Bakudankabe {
    /// `PARAMS_GET_U(params, 0, 6)`.
    pub fn switch_flag(&self) -> i32 {
        SWITCH_FLAG.get(self.actor.params) as i32
    }

    fn load_collision(play: &PlayState) -> Option<Arc<CollisionHeader>> {
        crate::obj_kibako2::load_collision(play, OBJECT, COLLISION)
    }

    /// `BgSpot08Bakudankabe_Init`: `DynaPolyActor_Init(0)`; its flag set, gone; else the spheres
    /// (`func_808B02D0`), `gZorasFountainBombableWallCol`, then `sInitChain` (scale 1, the
    /// culling volume 3200, 800, 1000).
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut this = BgSpot08Bakudankabe { actor, bg: BG_ACTOR_MAX, collider: ColliderJntSph::default() };
        if play.flags.get_switch(this.switch_flag()) {
            this.actor.kill();
            return Box::new(this);
        }
        // func_808B02D0: Collider_InitJntSph, Collider_SetJntSph.
        this.collider = ColliderJntSph::new(&JNT_SPH_INIT, &JNT_SPH_ELEMENTS);
        // DynaPoly_SetBgActor at Actor_Init's scale (0.01): the transform is read from the actor
        // every frame from then on (here, once the init chain has set it).
        if let Some(h) = Self::load_collision(play) {
            this.bg = play.col.dyna.set_bg_actor(h, source(&this.actor), 0);
        }
        // sInitChain: ICHAIN_VEC3F(scale, 1), cullingVolumeDistance 3200, cullingVolumeScale 800,
        // cullingVolumeDownward 1000.
        this.actor.scale = Vec3::splat(1.0);
        this.actor.culling_volume_distance = 3200.0;
        this.actor.culling_volume_scale = 800.0;
        this.actor.culling_volume_downward = 1000.0;
        play.col.dyna.set_source(this.bg, source(&this.actor));
        Box::new(this)
    }

    /// `func_808B0324`: the 24 pieces of rock up its face, then the dust at `D_808B08AC`'s points
    /// (see the module's docs).
    pub fn func_808b0324(&self, play: &mut PlayState) {
        let sin_y = sin_s(self.actor.shape_rot.y);
        let cos_y = cos_s(self.actor.shape_rot.y);
        let pos = self.actor.world_pos;
        play.with_ss(|ss| {
            for i in 0..24 {
                let temp1 = (ss.rand.zero_one() - 0.5) * 440.0;
                let temp2 = (ss.rand.zero_one() - 0.5) * 20.0;
                let burst_depth_y = Vec3::new(pos.x + temp2 * sin_y + (temp1 * cos_y), (pos.y + 20.0) + (i as f32 * (65.0 / 12.0)), pos.z + temp2 * cos_y - (temp1 * sin_y));
                let burst_depth_x = Vec3::new(0.0, (ss.rand.zero_one() - 0.2) * 12.0, 0.0);
                let scale = (ss.rand.zero_one() * 75.0 + 10.0) as i16;
                let gravity_influence = if scale < 25 {
                    -300
                } else if scale < 50 {
                    -360
                } else {
                    -420
                };
                let rotation_speed = if ss.rand.zero_one() < 0.4 { 65 } else { 33 };
                ss.kakera_spawn(burst_depth_y, burst_depth_x, burst_depth_y, gravity_influence, rotation_speed, 0x1E, 4, 0, scale, 1, 3, 80, KAKERA_COLOR_NONE, OBJECT_GAMEPLAY_FIELD_KEEP, Some(("gameplay_field_keep", "gFieldKakeraDL")));
            }
        });
        for d in &D_808B08AC {
            let p = Vec3::new(pos.x + d.z * sin_y + d.x * cos_y, pos.y + d.y, pos.z + d.z * cos_y - (d.x * sin_y));
            func_80033480(play, p, 120.0, 4, 0x78, 0xA0, 1);
        }
    }
}

impl ActorImpl for BgSpot08Bakudankabe {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `BgSpot08Bakudankabe_Update`: hit, it breaks (`func_808B0324`), its flag set, the wall's
    /// sound and the chime, gone; else within 800 across of Link its spheres set for hits.
    fn update(&mut self, play: &mut PlayState) {
        if self.collider.base.ac_flags & AC_HIT != 0 {
            self.func_808b0324(play);
            play.flags.set_switch(self.switch_flag());
            play.sfx_source_play_sfx_at_fixed_world_pos(self.actor.world_pos, 40, NA_SE_EV_WALL_BROKEN);
            play.audio.play_sfx_centered(NA_SE_SY_CORRECT_CHIME);
            self.actor.kill();
        } else if self.actor.xz_dist_to_player < 800.0 {
            play.collision_check_set_ac(&self.actor, 0, &mut self.collider);
        }
    }

    /// `BgSpot08Bakudankabe_Destroy`: the collision (nothing for the spheres).
    fn destroy(&mut self, play: &mut PlayState) {
        play.col.dyna.delete_bg_actor(self.bg);
    }

    fn dyna_bg_id(&self) -> Option<u16> {
        (self.bg != BG_ACTOR_MAX).then_some(self.bg)
    }

    /// `BgSpot08Bakudankabe_Draw`'s `Collider_UpdateSpheres(0, ..)`, `(1, ..)` and `(2, ..)`: at
    /// the actor's model matrix (only limb 0's spheres: all three).
    fn draw_update(&mut self, _play: &mut PlayState) {
        let m = actor_draw_matrix(&RenderState::of(&self.actor));
        for limb in 0..3 {
            self.collider.update_spheres(limb, &m);
        }
    }

    /// `BgSpot08Bakudankabe_Draw`: `Gfx_DrawDListOpa(gZorasFountainBombableWallDL)`.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        crate::gfx_draw_dlist_opa(out, OBJECT, DL, rs);
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
