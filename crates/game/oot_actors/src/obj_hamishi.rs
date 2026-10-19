//! `Obj_Hamishi` (`ovl_Obj_Hamishi/z_obj_hamishi.c`): a bronze boulder, the Megaton Hammer's: the
//! silver boulder's model (`gameplay_field_keep`'s `gSilverRockDL`, scale 0.4, 80 up its model)
//! tinted bronze (prim 255, 170, 130).
//!
//! Its cylinder (50 by 70, `COL_MATERIAL_HARD`: the sword bounces off) takes Player's weapons
//! (`0x4FC1FFF6`, which leaves out explosives and Deku Nuts) and pushes everything (OC), while Link
//! is within 600 across; it stays where it was placed (`Collider_UpdateCylinder` only in the
//! init). Only the hammer (`DMG_HAMMER`) does anything: the first hit shakes it for 15 frames
//! (`ObjHamishi_Shake`: 2 across and 400 of tilt, easing off, then back home), the second breaks
//! it (`ObjHamishi_Break`: nine bronze pieces of `gSilverRockFragmentsDL` (`EffectSsKakera`,
//! `sEffectScales`), two rings of dust (`func_80033480`)), `NA_SE_EV_WALL_BROKEN`, its switch flag
//! set, gone. Placed with no yaw, it's turned at random; in a cutscene it's seen 1000 further.
//!
//! The hammer is an adult's (Phase 15), so a child meets them as walls: solid, and neither his
//! sword nor a bomb breaks them. Goron City's are room 0's five, params 0x2A to 0x2C, 0x2F, 0x30
//! (temporary switch flags), in front of the silver rocks. The whole overlay is ported (the hammer
//! isn't: a test injects its hit).
//!
//! Params: `switchFlag` bits 0 to 5 (`PARAMS_GET_U(params, 0, 6)`: set when it breaks; set at
//! init, it's gone; Goron City's are the room's temporary flags 0x2A to 0x30, the crater's
//! 0x3F); bits 6 and 7 unused (the crater's 0xFF).
//! Flags: reads switch flag `switchFlag` (set: gone at init) and sets it when it breaks.

use glam::Vec3;
use oot_game::actor::Actor;
use oot_game::actor_ctx::{ACTORCAT_PROP, ActorImpl, ActorProfile, func_80033480};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind};
use oot_game::audio::sfx::NA_SE_EV_WALL_BROKEN;
use oot_game::collision_check::*;
use oot_game::effect::kakera::{KAKERA_COLOR_BROWN, OBJECT_GAMEPLAY_FIELD_KEEP};
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo, actor_draw_matrix};

/// `ACTOR_OBJ_HAMISHI` (`actor_table.h`: 0x01D2).
pub const ACTOR_OBJ_HAMISHI: i16 = 0x01D2;
pub const OBJECT: &str = "gameplay_field_keep";
const DL: &str = "gSilverRockDL";
const FRAGMENTS_DL: &str = "gSilverRockFragmentsDL";

/// `Obj_Hamishi_Profile`: `FLAGS` 0, `OBJECT_GAMEPLAY_FIELD_KEEP`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_OBJ_HAMISHI, name: "Obj_Hamishi", category: ACTORCAT_PROP, flags: 0, object: OBJECT, schema: &SCHEMA };

/// `PARAMS_GET_U(params, 0, 6)`: the switch flag.
pub const SWITCH_FLAG: Field = Field::params("switchFlag", 0, 6, Kind::SwitchFlag, "set when it breaks; set at init, it's gone (Goron City's are temporary, 0x2A to 0x30)");
/// Bits 6 and 7: the crater's 0xFF (flag 0x3F) sets them.
pub const SWITCH_FLAG_HIGH: Field = Field::params("switchFlag bits 6 and 7", 6, 2, Kind::Unused, "set with a switch flag of 0x3F (0xFF) in the data; read by nothing");

/// Obj_Hamishi's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema =
    ActorSchema { fields: &[SWITCH_FLAG, SWITCH_FLAG_HIGH], flags: &[FlagUse::new(Flag::Switch("switchFlag")).reads().sets().note("set: broken by the hammer, not spawned again")], todo: false, objects: &[], texts: &[] };

/// `sCylinderInit`: 50 by 70, hard, hit by `0x4FC1FFF6`, pushing (OC) every type.
pub const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_HARD, at_flags: AT_NONE, ac_flags: AC_ON | AC_HARD | AC_TYPE_PLAYER, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_2, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0x0000_0000, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x00 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0x4FC1_FFF6, hit_backlash: HIT_BACKLASH_NONE, defense: 0x00 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_ON,
    },
    dim: eng_collision::math3d::Cylinder16 { radius: 50, height: 70, y_shift: 0, pos: [0, 0, 0] },
};

/// `sColChkInfoInit`: health 0, 12 by 60, `MASS_IMMOVABLE`.
pub const COL_CHK_INFO_INIT: CollisionCheckInfoInit = CollisionCheckInfoInit { health: 0, cyl_radius: 12, cyl_height: 60, mass: MASS_IMMOVABLE };

/// `sEffectScales`: the nine pieces' sizes.
pub const EFFECT_SCALES: [i16; 9] = [145, 135, 115, 85, 75, 53, 45, 40, 35];

/// The bake: `gSilverRockDL` after `Gfx_SetupDL_25Opa` and the draw's
/// `gDPSetPrimColor(0, 0, 255, 170, 130, 255)` (G_SETPRIMCOLOR 0xFA, G_ENDDL 0xDF).
const BAKE: &str = "Obj_Hamishi/gSilverRockDL";
const SEG_COLOR: u8 = 0x0B;

/// `ObjHamishi_Draw`'s list.
pub fn bakes() -> Vec<MeshBake> {
    vec![MeshBake {
        name: BAKE.into(),
        object: OBJECT.into(),
        segments: vec![(SEG_COLOR, BakeSegment::Commands(vec![(0xFA00_0000, u32::from_be_bytes([255, 170, 130, 255])), (0xDF00_0000, 0)]))],
        prelude: vec![SEG_COLOR],
        body: BakeBody::DLists(vec![(OBJECT.into(), DL.into())]),
    }]
}

pub struct ObjHamishi {
    pub actor: Actor,
    pub collider: ColliderCylinder,
    pub shake_pos_size: f32,
    pub shake_rot_size: f32,
    pub shake_frames: i16,
    pub shake_pos_phase: i16,
    pub shake_rot_phase: i16,
    pub hit_count: i16,
}

impl ObjHamishi {
    /// `PARAMS_GET_U(params, 0, 6)`.
    fn switch_flag(&self) -> i32 {
        SWITCH_FLAG.get(self.actor.params) as i32
    }

    /// `ObjHamishi_Init`: `sInitChain` (scale 0.4; culling volume 2000, 250, 500; 1000 further in a
    /// cutscene), a random yaw if it has none, the cylinder at its placement
    /// (`ObjHamishi_InitCollision`), `sColChkInfoInit`; gone if its flag is set; else 80 up its
    /// model.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // sInitChain: ICHAIN_VEC3F_DIV1000(scale, 400); cullingVolumeDistance 2000,
        // cullingVolumeScale 250, cullingVolumeDownward 500.
        actor.scale = Vec3::splat(0.4);
        actor.culling_volume_distance = 2000.0;
        actor.culling_volume_scale = 250.0;
        actor.culling_volume_downward = 500.0;
        if play.cs_ctx.state != oot_game::cutscene::CS_STATE_IDLE {
            actor.culling_volume_distance += 1000.0;
        }
        if actor.shape_rot.y == 0 {
            let r = play.rand.zero_float(65536.0) as i32 as i16;
            actor.shape_rot.y = r;
            actor.world_rot.y = r;
            actor.home_rot.y = r;
        }
        // ObjHamishi_InitCollision: Collider_InitCylinder, Collider_SetCylinder,
        // Collider_UpdateCylinder.
        let mut collider = ColliderCylinder::new(&CYLINDER_INIT);
        collider.update(&actor);
        actor.col_chk_info.set_info(None, &COL_CHK_INFO_INIT);
        let mut this = ObjHamishi { actor, collider, shake_pos_size: 0.0, shake_rot_size: 0.0, shake_frames: 0, shake_pos_phase: 0, shake_rot_phase: 0, hit_count: 0 };
        if play.flags.get_switch(this.switch_flag()) {
            this.actor.kill();
            return Box::new(this);
        }
        this.actor.shape_y_offset = 80.0;
        Box::new(this)
    }

    /// `ObjHamishi_Shake`: while shaking, its place and tilt wobble (the phases on by 5000 and
    /// 0xE10, the sizes easing to 0 by 0.15 and 40); after, it steps back home (1 a frame) and
    /// level (`Math_ScaledStepToS` by 0xBB8).
    fn shake(&mut self) {
        let a = &mut self.actor;
        if self.shake_frames > 0 {
            self.shake_frames -= 1;
            self.shake_pos_phase = self.shake_pos_phase.wrapping_add(5000);
            self.shake_rot_phase = self.shake_rot_phase.wrapping_add(0xE10);
            eng_math::step_to_f(&mut self.shake_pos_size, 0.0, 0.15);
            eng_math::step_to_f(&mut self.shake_rot_size, 0.0, 40.0);
            a.world_pos.x = a.home_pos.x + (eng_math::sin_s(self.shake_pos_phase.wrapping_mul(4)) * self.shake_pos_size);
            a.world_pos.z = a.home_pos.z + (eng_math::cos_s(self.shake_pos_phase.wrapping_mul(7)) * self.shake_pos_size);
            a.shape_rot.x = a.home_rot.x.wrapping_add((eng_math::sin_s(self.shake_rot_phase.wrapping_mul(4)) * self.shake_rot_size) as i32 as i16);
            a.shape_rot.z = a.home_rot.z.wrapping_add((eng_math::cos_s(self.shake_rot_phase.wrapping_mul(7)) * self.shake_rot_size) as i32 as i16);
        } else {
            eng_math::step_to_f(&mut a.world_pos.x, a.home_pos.x, 1.0);
            eng_math::step_to_f(&mut a.world_pos.z, a.home_pos.z, 1.0);
            eng_math::scaled_step_to_s(&mut a.shape_rot.x, a.home_rot.x, 0xBB8);
            eng_math::scaled_step_to_s(&mut a.shape_rot.z, a.home_rot.z, 0xBB8);
        }
    }

    /// `ObjHamishi_Break`: nine bronze pieces (`sEffectScales`) round it a turn of 20000 apart from
    /// 1000 + 20000 (up to 10 out, 5 to 45 up), flying 2 to 12 out and up by `rand × i × 2.5 +
    /// rand × 15`; the first big and heavy (41, gravity -450), the next three medium (37, -380),
    /// the rest light (69, -320); then dust: six puffs 140 across, 180 big, and twelve 80 big.
    pub fn break_(&mut self, play: &mut PlayState) {
        let wp = self.actor.world_pos;
        play.with_ss(|ss| {
            let mut phi_s0: i16 = 1000;
            for (i, &scale) in EFFECT_SCALES.iter().enumerate() {
                phi_s0 = phi_s0.wrapping_add(20000);
                let temp_f20 = ss.rand.zero_one() * 10.0;
                let x = (eng_math::sin_s(phi_s0) * temp_f20) + wp.x;
                let y = (ss.rand.zero_one() * 40.0) + wp.y + 5.0;
                let z = (eng_math::cos_s(phi_s0) * temp_f20) + wp.z;
                let pos = Vec3::new(x, y, z);
                let temp_f20 = (ss.rand.zero_one() * 10.0) + 2.0;
                let vx = eng_math::sin_s(phi_s0) * temp_f20;
                let temp_f22 = ss.rand.zero_one();
                let vy = (ss.rand.zero_one() * i as f32 * 2.5) + (temp_f22 * 15.0);
                let vz = eng_math::cos_s(phi_s0) * temp_f20;
                let velocity = Vec3::new(vx, vy, vz);
                let (phi_v0, gravity) = if i == 0 {
                    (41, -450)
                } else if i < 4 {
                    (37, -380)
                } else {
                    (69, -320)
                };
                ss.kakera_spawn(pos, velocity, wp, gravity, phi_v0, 30, 5, 0, scale, 3, 0, 70, KAKERA_COLOR_BROWN, OBJECT_GAMEPLAY_FIELD_KEEP, Some((OBJECT, FRAGMENTS_DL)));
            }
        });
        func_80033480(play, wp, 140.0, 6, 180, 90, 1);
        func_80033480(play, wp, 140.0, 12, 80, 90, 1);
    }
}

impl ActorImpl for ObjHamishi {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `ObjHamishi_Update`: the shake; a hammer's hit (`DMG_HAMMER`) shakes it the first time
    /// (15 frames, 2 across, 400 of tilt) and breaks it the second (the pieces,
    /// `NA_SE_EV_WALL_BROKEN` for 40 frames at it, its flag, gone). Otherwise last frame's hit is
    /// cleared and, Link within 600 across, its cylinder is set for hits and pushes.
    fn update(&mut self, play: &mut PlayState) {
        self.shake();
        let hammer = self.collider.base.ac_flags & AC_HIT != 0 && self.collider.info.ac_hit_elem.is_some_and(|h| h.at_dmg_info.dmg_flags & DMG_HAMMER != 0);
        if hammer {
            self.collider.base.ac_flags &= !AC_HIT;
            self.hit_count += 1;
            if self.hit_count < 2 {
                self.shake_frames = 15;
                self.shake_pos_size = 2.0;
                self.shake_rot_size = 400.0;
            } else {
                self.break_(play);
                play.sfx_source_play_sfx_at_fixed_world_pos(self.actor.world_pos, 40, NA_SE_EV_WALL_BROKEN);
                play.flags.set_switch(self.switch_flag());
                self.actor.kill();
            }
        } else {
            self.collider.base.ac_flags &= !AC_HIT;
            if self.actor.xz_dist_to_player < 600.0 {
                play.collision_check_set_ac(&self.actor, 0, &mut self.collider);
                play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
            }
        }
    }

    /// `ObjHamishi_Destroy`: the cylinder (nothing to free).
    fn destroy(&mut self, _play: &mut PlayState) {}

    /// `ObjHamishi_Draw`: `Gfx_SetupDL_25Opa`, the model matrix, prim (255, 170, 130, 255),
    /// `gSilverRockDL`.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        out.opa.push(eng_gfx::DrawCmd::new(eng_gfx::MeshKey::named(keys::bake(BAKE)), actor_draw_matrix(rs)));
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
