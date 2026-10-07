//! `Obj_Bombiwa` (`ovl_Obj_Bombiwa/z_obj_bombiwa.c`): the round brown boulder a bomb or the
//! hammer breaks, of `object_bombiwa`, drawn with `object_bombiwa_DL_0009E0`.
//!
//! Params: bits 0..5 the switch flag set once it's broken (a broken one isn't spawned again);
//! bit 15 plays the chime when it breaks. Placed with no yaw, it's turned at random
//! (`Rand_ZeroFloat(65536)`); it sits 20 above its placement, drawn 200 (model units) lower.
//!
//! Its cylinder (55 by 70, `COL_MATERIAL_HARD`: the sword bounces off) takes hits from Player's
//! weapons (`0x4FC1FFFE`) and pushes actors off (OC) within 800 of Link. An exploding explosive
//! within reach (`func_80033684`: a bomb's blast) or a hammer's hit (`DMG_HAMMER`) breaks it:
//! eight fragments (`EffectSsKakera`, its own list), nine puffs of dust (`func_80033480`), its
//! flag, `NA_SE_EV_WALL_BROKEN`, the chime for bit 15, gone.
//!
//! The Master Quest Deku Tree's are room 2's three, params 3, 4 and 8 (flags 3, 4, 8; no chime)
//! at y 656, turned -24576 (so no `Rand`). Link has no bombs or hammer yet (GAME-05 milestone
//! 4c): the tests inject the explosion and the hit. The whole overlay is ported; the cull zone
//! isn't, for any actor.

use glam::Vec3;
use oot_game::actor::Actor;
use oot_game::actor_ctx::{ACTORCAT_PROP, ActorImpl, ActorProfile, func_80033480, func_80033684};
use oot_game::audio::sfx::{NA_SE_EV_WALL_BROKEN, NA_SE_SY_CORRECT_CHIME};
use oot_game::collision_check::*;
use oot_game::effect::kakera::KAKERA_COLOR_NONE;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};

/// `ACTOR_OBJ_BOMBIWA` (`actor_table.h`: 0x0127).
pub const ACTOR_OBJ_BOMBIWA: i16 = 0x0127;
pub const OBJECT: &str = "object_bombiwa";
/// `OBJECT_BOMBIWA` (`object_table.h`: 0x0163).
pub const OBJECT_BOMBIWA: i16 = 0x0163;
/// `object_bombiwa_DL_0009E0`: the boulder, and each of its fragments.
pub const DL: &str = "object_bombiwa_DL_0009E0";

/// `Obj_Bombiwa_Profile`: `FLAGS` 0.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_OBJ_BOMBIWA, name: "Obj_Bombiwa", category: ACTORCAT_PROP, flags: 0, object: OBJECT };

/// `sCylinderInit`: 55 by 70, hard, hit by `0x4FC1FFFE`, pushing (OC) every type.
pub const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit {
        col_type: COL_MATERIAL_HARD,
        at_flags: AT_NONE,
        ac_flags: AC_ON | AC_HARD | AC_TYPE_PLAYER,
        oc_flags1: OC1_ON | OC1_TYPE_ALL,
        oc_flags2: OC2_TYPE_2,
        shape: COLSHAPE_CYLINDER,
    },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0x0000_0000, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x00 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0x4FC1_FFFE, hit_backlash: HIT_BACKLASH_NONE, defense: 0x00 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_ON,
    },
    dim: eng_collision::math3d::Cylinder16 { radius: 55, height: 70, y_shift: 0, pos: [0, 0, 0] },
};

/// `sColChkInfoInit`: health 0, 12 by 60, `MASS_IMMOVABLE`.
pub const COL_CHK_INFO_INIT: CollisionCheckInfoInit = CollisionCheckInfoInit { health: 0, cyl_radius: 12, cyl_height: 60, mass: MASS_IMMOVABLE };

/// `sEffectScales`: the eight fragments' sizes.
pub const EFFECT_SCALES: [i16; 8] = [17, 14, 10, 8, 7, 5, 3, 2];

pub struct ObjBombiwa {
    pub actor: Actor,
    pub collider: ColliderCylinder,
}

impl ObjBombiwa {
    /// `PARAMS_GET_U(params, 0, 6)`: the switch flag.
    pub fn switch_flag(&self) -> i32 {
        (self.actor.params as u16 & 0x3F) as i32
    }

    /// `ObjBombiwa_Init`: scale 0.1 (`sInitChain`), the cylinder at its placement
    /// (`ObjBombiwa_InitCollision`); gone if its flag is set; else `sColChkInfoInit`, a random yaw
    /// if it has none, `shape.yOffset` -200, 20 up.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // sInitChain: ICHAIN_VEC3F_DIV1000(scale, 100); cullingVolumeDistance 2000,
        // cullingVolumeScale 350, cullingVolumeDownward 1000 (the cull zone isn't ported).
        actor.scale = Vec3::splat(0.1);
        // ObjBombiwa_InitCollision: Collider_InitCylinder, Collider_SetCylinder,
        // Collider_UpdateCylinder (before the 20 up: the cylinder stays there, never updated).
        let mut collider = ColliderCylinder::new(&CYLINDER_INIT);
        collider.update(&actor);
        let mut b = ObjBombiwa { actor, collider };
        if play.flags.get_switch(b.switch_flag()) {
            b.actor.kill();
        } else {
            b.actor.col_chk_info.set_info(None, &COL_CHK_INFO_INIT);
            if b.actor.shape_rot.y == 0 {
                let rand = play.rand.zero_float(65536.0) as i32 as i16;
                b.actor.world_rot.y = rand;
                b.actor.shape_rot.y = rand;
            }
            b.actor.shape_y_offset = -200.0;
            b.actor.world_pos.y = b.actor.home_pos.y + 20.0;
        }
        Box::new(b)
    }

    /// `ObjBombiwa_Break`: eight fragments (`sEffectScales`) within 5 across of its home and 8 to
    /// 13 up, flying out up to 7.5 each way and up at 5 to 21; the big ones (11 and over) tumbling
    /// medium (37), the rest slow (33); then nine puffs of dust 60 across (`func_80033480`, 100
    /// big, lit).
    pub fn break_(&mut self, play: &mut PlayState) {
        let home = self.actor.home_pos;
        play.with_ss(|ss| {
            for &scale in &EFFECT_SCALES {
                let x = ((ss.rand.zero_one() - 0.5) * 10.0) + home.x;
                let y = ((ss.rand.zero_one() * 5.0) + home.y) + 8.0;
                let z = ((ss.rand.zero_one() - 0.5) * 10.0) + home.z;
                let pos = Vec3::new(x, y, z);
                let vx = (ss.rand.zero_one() - 0.5) * 15.0;
                let vy = (ss.rand.zero_one() * 16.0) + 5.0;
                let vz = (ss.rand.zero_one() - 0.5) * 15.0;
                let velocity = Vec3::new(vx, vy, vz);
                let arg5 = if scale >= 11 { 37 } else { 33 };
                ss.kakera_spawn(pos, velocity, pos, -400, arg5, 10, 2, 0, scale, 1, 0, 80, KAKERA_COLOR_NONE, OBJECT_BOMBIWA, Some((OBJECT, DL)));
            }
        });
        func_80033480(play, self.actor.world_pos, 60.0, 8, 100, 160, 1);
    }
}

impl ActorImpl for ObjBombiwa {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `ObjBombiwa_Update`: an explosion in reach or a hammer's hit breaks it, sets its flag, with
    /// `NA_SE_EV_WALL_BROKEN` (80 frames at its position) and bit 15's chime, and it's gone; else
    /// within 800 of Link its cylinder is set for hits and pushes.
    fn update(&mut self, play: &mut PlayState) {
        let hammer = self.collider.base.ac_flags & AC_HIT != 0 && self.collider.info.ac_hit_elem.is_some_and(|h| h.at_dmg_info.dmg_flags & DMG_HAMMER != 0);
        if func_80033684(play, play.cur_actor, &self.actor).is_some() || hammer {
            self.break_(play);
            play.flags.set_switch(self.switch_flag());
            play.sfx_source_play_sfx_at_fixed_world_pos(self.actor.world_pos, 80, NA_SE_EV_WALL_BROKEN);
            if self.actor.params as u16 >> 15 != 0 {
                play.audio.play_sfx_centered(NA_SE_SY_CORRECT_CHIME);
            }
            self.actor.kill();
        } else {
            self.collider.base.ac_flags &= !AC_HIT;
            if self.actor.xz_dist_to_player < 800.0 {
                play.collision_check_set_ac(&self.actor, 0, &mut self.collider);
                play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
            }
        }
    }

    /// `ObjBombiwa_Destroy`: the cylinder (nothing to free).
    fn destroy(&mut self, _play: &mut PlayState) {}

    /// `ObjBombiwa_Draw`: `Gfx_DrawDListOpa(object_bombiwa_DL_0009E0)`.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        crate::gfx_draw_dlist_opa(out, OBJECT, DL, rs);
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
