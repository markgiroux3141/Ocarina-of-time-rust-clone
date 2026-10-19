//! `Bg_Bombwall` (`ovl_Bg_Bombwall/z_bg_bombwall.c`): a cracked rock wall of
//! `gameplay_field_keep` that a bomb or the hammer opens, over a hidden way in (Death Mountain
//! Trail's three: a Gold Skulltula's nook, a chest's, the Great Fairy's fountain; the crater's
//! two). A DynaPoly actor (`gBgBombwallCol`) at scale 0.1, drawn `gBgBombwallNormalDL`, with
//! three triangles over its face (`sTrisElementsInit`, 2 in front of it) that take
//! `DMG_HAMMER_JUMP | DMG_HAMMER_SWING | DMG_EXPLOSIVE` (`0x40000048`), set for hits while Link
//! is within 600 (x and z).
//!
//! Hit: eight puffs of dust over its face (`func_8086EB5C`), its switch flag set; two frames on
//! (`unk_2A0` 1, counted down) it's broken: `gBgBombwallBrokenDL`, its collision and triangles
//! gone, and bit 15's chime (`NA_SE_SY_CORRECT_CHIME`). With its flag set at init it's broken
//! from the start, with no collision.
//!
//! The whole overlay is ported.
//!
//! Params: `switchFlag` bits 0 to 5 (set when it's hit; set at init, it's broken); bits 6 to 14
//! unused; `chime` bit 15 (1: the chime as it breaks; the trail's 0x8001, 0x8002, 0x8003, the
//! crater's 0x0000 and 0x0001 without).
//! Flags: reads and sets switch flag `switchFlag`.

use std::sync::Arc;

use eng_collision::collision::CollisionHeader;
use eng_collision::dyna::{BG_ACTOR_MAX, BgActorSource};
use eng_math::{cos_s, sin_s};
use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_IGNORE_POINT_LIGHTS, Actor};
use oot_game::actor_ctx::{ACTORCAT_BG, ActorImpl, ActorProfile, func_80033480};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind};
use oot_game::audio::sfx::NA_SE_SY_CORRECT_CHIME;
use oot_game::collision_check::*;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};

/// `ACTOR_BG_BOMBWALL` (`actor_table.h`: 0x00D0).
pub const ACTOR_BG_BOMBWALL: i16 = 0x00D0;
/// `OBJECT_GAMEPLAY_FIELD_KEEP` (`object_table.h`: 0x0002).
pub const OBJECT: &str = "gameplay_field_keep";
pub const COLLISION: &str = "gBgBombwallCol";
pub const NORMAL_DL: &str = "gBgBombwallNormalDL";
pub const BROKEN_DL: &str = "gBgBombwallBrokenDL";

/// `Bg_Bombwall_Profile`: `ACTORCAT_BG`, `ACTOR_FLAG_IGNORE_POINT_LIGHTS`,
/// `OBJECT_GAMEPLAY_FIELD_KEEP`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_BG_BOMBWALL, name: "Bg_Bombwall", category: ACTORCAT_BG, flags: ACTOR_FLAG_IGNORE_POINT_LIGHTS, object: OBJECT, schema: &SCHEMA };

/// `PARAMS_GET_U(params, 0, 6)`: the switch flag.
pub const SWITCH_FLAG: Field = Field::params("switchFlag", 0, 6, Kind::SwitchFlag, "set when it's hit; set at init, it's broken");
/// `PARAMS_GET_U(params, 15, 1)`: the chime.
pub const CHIME: Field = Field::params("chime", 15, 1, Kind::Named(&[(0, "no chime"), (1, "the chime (NA_SE_SY_CORRECT_CHIME)")]), "the chime as it breaks");

/// Bg_Bombwall's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[SWITCH_FLAG, CHIME],
    flags: &[FlagUse::new(Flag::Switch("switchFlag")).reads().sets().note("set as it's hit; set at init, it's broken")],
    todo: false,
    objects: &[],
    texts: &[],
};

/// One of `sTrisElementsInit`'s elements: hit by `0x40000048`.
const fn tris_element(vtx: [Vec3; 3]) -> ColliderTrisElementInit {
    ColliderTrisElementInit {
        info: ColliderElementInit {
            elem_material: ELEM_MATERIAL_UNK0,
            at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0x0000_0000, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x00 },
            ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0x4000_0048, hit_backlash: HIT_BACKLASH_NONE, defense: 0x00 },
            at_elem_flags: ATELEM_NONE,
            ac_elem_flags: ACELEM_ON,
            oc_elem_flags: OCELEM_NONE,
        },
        vtx,
    }
}

/// `sTrisElementsInit`: three triangles over the wall's face, 140 wide and 180 tall, to a point 30
/// in front at its foot (model units: world units, not scaled).
pub const TRIS_ELEMENTS: [ColliderTrisElementInit; 3] = [
    tris_element([Vec3::new(-70.0, 176.0, 0.0), Vec3::new(-70.0, -4.0, 0.0), Vec3::new(0.0, -4.0, 30.0)]),
    tris_element([Vec3::new(70.0, 176.0, 0.0), Vec3::new(-70.0, 176.0, 0.0), Vec3::new(0.0, -4.0, 30.0)]),
    tris_element([Vec3::new(70.0, -4.0, 0.0), Vec3::new(70.0, 176.0, 0.0), Vec3::new(0.0, -4.0, 30.0)]),
];

/// `sTrisInit`: no material, hit by Player's.
pub const TRIS_INIT: ColliderInit = ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_NONE, ac_flags: AC_ON | AC_TYPE_PLAYER, oc_flags1: OC1_NONE, oc_flags2: OC2_NONE, shape: COLSHAPE_TRIS };

/// `D_8086F010`: the seven puffs of dust over the face (x across, y up, z out).
pub const D_8086F010: [[i16; 3]; 7] = [[40, 85, 21], [-43, 107, 14], [-1, 142, 14], [-27, 44, 27], [28, 24, 20], [-39, 54, 21], [49, 50, 20]];

/// `unk_2A2`'s bits: 2 the DynaPoly registered, 1 the triangles set.
const DYNA_SET: u8 = 2;
const TRIS_SET: u8 = 1;

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `NULL`: broken.
    None,
    /// `func_8086ED70`: whole, waiting for a hit.
    Wait,
    /// `func_8086EE40`: hit, breaking.
    Break,
}

pub struct BgBombwall {
    /// `dyna.actor`, `dyna.bgId`.
    pub actor: Actor,
    pub bg: u16,
    pub collider: ColliderTris,
    pub action: Action,
    /// `dList`.
    pub dlist: &'static str,
    /// `unk_2A0`: the frames before it breaks.
    pub unk_2a0: i16,
    /// `unk_2A2`: what's set up (`DYNA_SET`, `TRIS_SET`).
    pub unk_2a2: u8,
}

fn source(a: &Actor) -> BgActorSource {
    let r = a.shape_rot;
    BgActorSource { pos: a.world_pos, shape_rot: [r.x, r.y, r.z], scale: a.scale, shape_y_offset: a.shape_y_offset }
}

/// `BgBombwall_RotateVec`: `arg1` turned about y by the sine `arg2` and cosine `arg3`.
pub fn rotate_vec(arg1: Vec3, arg2: f32, arg3: f32) -> Vec3 {
    Vec3::new((arg1.z * arg2) + (arg1.x * arg3), arg1.y, (arg1.z * arg3) - (arg1.x * arg2))
}

impl BgBombwall {
    /// `PARAMS_GET_U(params, 0, 6)`.
    pub fn switch_flag(&self) -> i32 {
        SWITCH_FLAG.get(self.actor.params) as i32
    }

    /// `BgBombwall_Init`: the culling volume and scale 0.1 (`sInitChain`). Its flag set: broken
    /// (`func_8086EE94`). Else its collision (`BgBombwall_InitDynapoly`), its triangles turned by its
    /// facing at its position (each 2 further out), waiting (`func_8086ED50`).
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut this = BgBombwall { actor, bg: BG_ACTOR_MAX, collider: ColliderTris::default(), action: Action::None, dlist: NORMAL_DL, unk_2a0: 0, unk_2a2: 0 };
        let sin = sin_s(this.actor.shape_rot.y);
        let cos = cos_s(this.actor.shape_rot.y);
        // sInitChain: cullingVolumeDistance 1800, cullingVolumeScale 300, cullingVolumeDownward 1000.
        this.actor.culling_volume_distance = 1800.0;
        this.actor.culling_volume_scale = 300.0;
        this.actor.culling_volume_downward = 1000.0;
        this.actor.scale = Vec3::splat(0.1);
        if play.flags.get_switch(this.switch_flag()) {
            this.func_8086ee94(play);
        } else {
            this.init_dynapoly(play);
            this.unk_2a2 |= DYNA_SET;
            this.collider = ColliderTris::new(&TRIS_INIT, &TRIS_ELEMENTS);
            for (i, e) in TRIS_ELEMENTS.iter().enumerate() {
                let vecs = e.vtx.map(|v| rotate_vec(Vec3::new(v.x, v.y, v.z + 2.0), sin, cos) + this.actor.world_pos);
                this.collider.set_vertices(i, vecs[0], vecs[1], vecs[2]);
            }
            this.unk_2a2 |= TRIS_SET;
            this.func_8086ed50();
        }
        log::debug!("(field keep general purpose bomb wall)(arg_data {:#06x})(angY {})", this.actor.params as u16, this.actor.shape_rot.y); // schema: printed whole
        Box::new(this)
    }

    fn load_collision(play: &PlayState) -> Option<Arc<CollisionHeader>> {
        crate::obj_kibako2::load_collision(play, OBJECT, COLLISION)
    }

    /// `BgBombwall_InitDynapoly`: `DynaPolyActor_Init(0)`, `gBgBombwallCol` registered.
    fn init_dynapoly(&mut self, play: &mut PlayState) {
        if let Some(h) = Self::load_collision(play) {
            self.bg = play.col.dyna.set_bg_actor(h, source(&self.actor), 0);
        }
        if self.bg == BG_ACTOR_MAX {
            log::warn!("Warning : move BG registration failed(../z_bg_bombwall.c 243)(arg_data {:#06x})", self.actor.params as u16); // schema: printed whole
        }
    }

    /// `BgBombwall_DestroyCollision`: the collision and the triangles, whichever are set.
    fn destroy_collision(&mut self, play: &mut PlayState) {
        if self.unk_2a2 & DYNA_SET != 0 {
            play.col.dyna.delete_bg_actor(self.bg);
            self.bg = BG_ACTOR_MAX;
            self.unk_2a2 &= !DYNA_SET;
        }
        if self.unk_2a2 & TRIS_SET != 0 {
            // Collider_DestroyTris: nothing to free.
            self.unk_2a2 &= !TRIS_SET;
        }
    }

    /// `func_8086EB5C`: seven puffs of dust (`func_80033480`, 50 across, three each, 20 to 139
    /// big growing 20 to 259, lit) at `D_8086F010`'s points turned by its facing; and one more 90
    /// up and 15 along +z (not turned), 40 across, five, 10 big growing 50.
    fn func_8086eb5c(&self, play: &mut PlayState) {
        let sin = sin_s(self.actor.shape_rot.y);
        let cos = cos_s(self.actor.shape_rot.y);
        let pos = self.actor.world_pos;
        for d in &D_8086F010 {
            let new_var = d[0] as f32;
            let temp = new_var * cos;
            let sp88 = Vec3::new(((sin * d[2] as f32) + temp) + pos.x, pos.y + d[1] as f32, ((d[2] as f32 * cos) - (sin * d[0] as f32)) + pos.z);
            let rand = ((play.rand.zero_one() * 120.0) as i16).wrapping_add(0x14);
            let rand2 = ((play.rand.zero_one() * 240.0) as i16).wrapping_add(0x14);
            func_80033480(play, sp88, 50.0, 2, rand, rand2, 1);
        }
        let sp88 = Vec3::new(pos.x, pos.y + 90.0, pos.z + 15.0);
        func_80033480(play, sp88, 40.0, 4, 0xA, 0x32, 1);
    }

    /// `func_8086ED50`: whole, waiting.
    fn func_8086ed50(&mut self) {
        self.dlist = NORMAL_DL;
        self.action = Action::Wait;
    }

    /// `func_8086ED70`: hit: breaking (`func_8086EDFC`), its switch flag set. Else within 600
    /// (x and z) of Link its triangles set for hits.
    fn func_8086ed70(&mut self, play: &mut PlayState) {
        if self.collider.base.ac_flags & AC_HIT != 0 {
            self.collider.base.ac_flags &= !AC_HIT;
            self.func_8086edfc(play);
            play.flags.set_switch(self.switch_flag());
        } else if self.actor.xz_dist_to_player < 600.0 {
            play.collision_check_set_ac(&self.actor, 0, &mut self.collider);
        }
    }

    /// `func_8086EDFC`: one frame to wait, the dust, breaking.
    fn func_8086edfc(&mut self, play: &mut PlayState) {
        self.dlist = NORMAL_DL;
        self.unk_2a0 = 1;
        self.func_8086eb5c(play);
        self.action = Action::Break;
    }

    /// `func_8086EE40`: the wait counted down; then broken, with bit 15's chime.
    fn func_8086ee40(&mut self, play: &mut PlayState) {
        if self.unk_2a0 > 0 {
            self.unk_2a0 -= 1;
        } else {
            self.func_8086ee94(play);
            if CHIME.get(self.actor.params) != 0 {
                play.audio.play_sfx_centered(NA_SE_SY_CORRECT_CHIME);
            }
        }
    }

    /// `func_8086EE94`: `gBgBombwallBrokenDL`, its collision gone, nothing more to do.
    fn func_8086ee94(&mut self, play: &mut PlayState) {
        self.dlist = BROKEN_DL;
        self.destroy_collision(play);
        self.action = Action::None;
    }
}

impl ActorImpl for BgBombwall {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `BgBombwall_Update`: the action, if any.
    fn update(&mut self, play: &mut PlayState) {
        match self.action {
            Action::None => {}
            Action::Wait => self.func_8086ed70(play),
            Action::Break => self.func_8086ee40(play),
        }
    }

    /// `BgBombwall_Destroy`: `BgBombwall_DestroyCollision`.
    fn destroy(&mut self, play: &mut PlayState) {
        self.destroy_collision(play);
    }

    fn dyna_bg_id(&self) -> Option<u16> {
        (self.bg != BG_ACTOR_MAX).then_some(self.bg)
    }

    /// `BgBombwall_Draw`: `Gfx_DrawDListOpa(dList)`.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        crate::gfx_draw_dlist_opa(out, OBJECT, self.dlist, rs);
    }

    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        (id == 0 && self.unk_2a2 & TRIS_SET != 0).then_some(ColliderMut::Tris(&mut self.collider))
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotate_vec_turns_about_y() {
        // Facing +x (sin 1, cos 0): +z goes to +x, +x to -z.
        let v = rotate_vec(Vec3::new(1.0, 2.0, 3.0), 1.0, 0.0);
        assert_eq!(v, Vec3::new(3.0, 2.0, -1.0));
    }
}
