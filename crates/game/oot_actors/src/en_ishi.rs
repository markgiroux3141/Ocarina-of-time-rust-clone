//! `En_Ishi` (`ovl_En_Ishi/z_en_ishi.c`): a small grey rock (`params & 1` 0, `gFieldKakeraDL`)
//! or a silver boulder (1, `gSilverRockDL`). The sword bounces off it (`AC_HARD`); a hammer or an
//! explosion breaks a small one.
//!
//! A broken small rock drops from its table (`EnIshi_DropCollectible`), and breaks into pieces
//! (`sFragmentSpawnFuncs`: `EffectSsKakera`) and dust (`sDustSpawnFuncs`: `func_80033480`); both
//! kinds' are ported (the large rock's come from `EnIshi_Fly`).
//!
//! Not ported: lifting and throwing (Player can't lift yet, so `Actor_HasParent` is never
//! true and the `EnIshi_LiftedUp` / `EnIshi_Fly` states are never reached).

use eng_collision::math3d::Cylinder16;
use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_THROW_ONLY, Actor, BGCHECKFLAG_GROUND, BGCHECKFLAG_WALL};
use oot_game::actor_ctx::{ACTORCAT_PROP, ActorImpl, ActorProfile, func_80033480};
use oot_game::collision_check::*;
use oot_game::effect::kakera::{KAKERA_COLOR_NONE, KAKERA_COLOR_WHITE, OBJECT_GAMEPLAY_FIELD_KEEP};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};

pub const ACTOR_EN_ISHI: i16 = 0x014E;

/// `En_Ishi_Profile`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_ISHI, name: "En_Ishi", category: ACTORCAT_PROP, flags: ACTOR_FLAG_THROW_ONLY, object: "gameplay_field_keep" };

/// `ROCK_SMALL`, `ROCK_LARGE`.
const ROCK_SMALL: usize = 0;
const ROCK_LARGE: usize = 1;

/// `sRockScales`, `D_80A7FA20` (the model's `yOffset`).
const ROCK_SCALES: [f32; 2] = [0.1, 0.4];
const Y_OFFSETS: [f32; 2] = [58.0, 80.0];

/// `sInitChains[type]`: gravity and `minVelocityY` (the cull zone isn't ported).
const GRAVITY: [f32; 2] = [-1.2, -2.5];
const MIN_VELOCITY_Y: f32 = -20.0;

/// `sCylinderInits`.
const CYLINDER_INITS: [ColliderCylinderInit; 2] = [
    ColliderCylinderInit {
        base: ColliderInit { col_type: COL_MATERIAL_HARD, at_flags: AT_NONE, ac_flags: AC_ON | AC_HARD | AC_TYPE_PLAYER, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_2, shape: COLSHAPE_CYLINDER },
        info: ColliderElementInit {
            elem_material: ELEM_MATERIAL_UNK0,
            at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: 0, damage: 0 },
            ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0x4FC1_FFFE, hit_backlash: 0, defense: 0 },
            at_elem_flags: ATELEM_NONE,
            ac_elem_flags: ACELEM_ON,
            oc_elem_flags: OCELEM_ON,
        },
        dim: Cylinder16 { radius: 10, height: 18, y_shift: -2, pos: [0; 3] },
    },
    ColliderCylinderInit {
        base: ColliderInit { col_type: COL_MATERIAL_HARD, at_flags: AT_NONE, ac_flags: AC_ON | AC_HARD | AC_TYPE_PLAYER, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_2, shape: COLSHAPE_CYLINDER },
        info: ColliderElementInit {
            elem_material: ELEM_MATERIAL_UNK0,
            at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: 0, damage: 0 },
            ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0x4FC1_FFF6, hit_backlash: 0, defense: 0 },
            at_elem_flags: ATELEM_NONE,
            ac_elem_flags: ACELEM_ON,
            oc_elem_flags: OCELEM_ON,
        },
        dim: Cylinder16 { radius: 55, height: 70, y_shift: 0, pos: [0; 3] },
    },
];

/// `sColChkInfoInit`.
const COL_CHK_INFO_INIT: CollisionCheckInfoInit = CollisionCheckInfoInit { health: 0, cyl_radius: 12, cyl_height: 60, mass: MASS_IMMOVABLE };

/// `DMG_HAMMER | DMG_EXPLOSIVE`: what breaks a small rock.
const DMG_BREAKS_ROCK: u32 = DMG_HAMMER_SWING | DMG_HAMMER_JUMP | DMG_EXPLOSIVE;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    /// `EnIshi_Wait`.
    Wait,
}

pub struct EnIshi {
    pub actor: Actor,
    pub collider: ColliderCylinder,
    action: Action,
}

impl EnIshi {
    fn ty(&self) -> usize {
        (self.actor.params & 1) as usize
    }

    /// `EnIshi_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let ty = (actor.params & 1) as usize;
        actor.gravity = GRAVITY[ty];
        actor.min_velocity_y = MIN_VELOCITY_Y;
        if actor.shape_rot.y == 0 {
            let r = play.rand.zero_float(65536.0) as i32 as i16;
            actor.shape_rot.y = r;
            actor.world_rot.y = r;
        }
        actor.scale = Vec3::splat(ROCK_SCALES[ty]);
        // EnIshi_InitCollider.
        let mut collider = ColliderCylinder::new(&CYLINDER_INITS[ty]);
        collider.update(&actor);
        let flag = ((actor.params as u16 >> 0xA) & 0x3C) | ((actor.params as u16 >> 6) & 3);
        if ty == ROCK_LARGE && play.flags.get_switch(flag as i32) {
            actor.kill();
            return Box::new(EnIshi { actor, collider, action: Action::Wait });
        }
        actor.col_chk_info.set_info(None, &COL_CHK_INFO_INIT);
        actor.shape_y_offset = Y_OFFSETS[ty];
        if (actor.params >> 5) & 1 == 0 && !snap_to_floor(&mut actor, play, 0.0) {
            actor.kill();
        }
        Box::new(EnIshi { actor, collider, action: Action::Wait })
    }

    /// `EnIshi_Wait`.
    fn wait(&mut self, play: &mut PlayState) {
        let ty = self.ty();
        // Actor_HasParent: Player doesn't lift things yet.
        let hit = self.collider.base.ac_flags & AC_HIT != 0;
        if hit && ty == ROCK_SMALL && self.collider.info.ac_hit_elem.is_some_and(|h| h.at_dmg_info.dmg_flags & DMG_BREAKS_ROCK != 0) {
            // EnIshi_DropCollectible.
            let mut drop_params = (self.actor.params >> 8) & 0xF;
            if drop_params >= 0xD {
                drop_params = 0;
            }
            crate::en_item00::item_drop_collectible_random(play, None, self.actor.world_pos, drop_params << 4);
            play.sfx_source_play_sfx_at_fixed_world_pos(self.actor.world_pos, BREAK_SFX_DURATIONS[ty], BREAK_SFX_IDS[ty]);
            self.spawn_fragments(play, ty);
            self.spawn_dust(play, ty);
            self.actor.kill();
        } else if self.actor.xz_dist_to_player < 600.0 {
            self.collider.update(&self.actor);
            self.collider.base.ac_flags &= !AC_HIT;
            play.collision_check_set_ac(&self.actor, 0, &mut self.collider);
            if self.actor.xz_dist_to_player < 400.0 {
                play.collision_check_set_oc(&self.actor, 0, &mut self.collider);
                // < 90: Actor_OfferGetItem offers the rock to Player's lift (not ported).
            }
        }
    }
}

/// `EnIshi_SpawnFragmentsSmall`'s `scales`.
pub const FRAGMENT_SCALES_SMALL: [i16; 6] = [16, 13, 11, 9, 7, 5];
/// `EnIshi_SpawnFragmentsLarge`'s `scales`.
pub const FRAGMENT_SCALES_LARGE: [i16; 9] = [145, 135, 120, 100, 70, 50, 45, 40, 35];

impl EnIshi {
    /// `sFragmentSpawnFuncs[type]`.
    pub fn spawn_fragments(&self, play: &mut PlayState, ty: usize) {
        if ty == ROCK_SMALL { self.spawn_fragments_small(play) } else { self.spawn_fragments_large(play) }
    }

    /// `sDustSpawnFuncs[type]`.
    pub fn spawn_dust(&self, play: &mut PlayState, ty: usize) {
        if ty == ROCK_SMALL { self.spawn_dust_small(play) } else { self.spawn_dust_large(play) }
    }

    /// The rock's velocity bounced off the ground (`BGCHECKFLAG_GROUND`: across `xz`, up `-y`)
    /// or a wall (`BGCHECKFLAG_WALL`: back `-xz`, `y`).
    fn bounced_velocity(&self, xz: f32, y: f32) -> Vec3 {
        let mut velocity = self.actor.velocity;
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
            velocity.x *= xz;
            velocity.y *= -y;
            velocity.z *= xz;
        } else if self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 {
            velocity.x *= -xz;
            velocity.y *= y;
            velocity.z *= -xz;
        }
        velocity
    }

    /// `EnIshi_SpawnFragmentsSmall`: six chips of `gFieldKakeraDL` (`scales`), within 4 across
    /// and 5 to 10 up, with the rock's bounced velocity ±5.5 across and up to 6 up; bouncing
    /// (collision mode 1, three bounces, radius 10), tumbling (`0x41` or `0x21`), gravity
    /// -420/256, 40 frames.
    fn spawn_fragments_small(&self, play: &mut PlayState) {
        let wp = self.actor.world_pos;
        let bounced = self.bounced_velocity(0.8, 0.8);
        play.with_ss(|ss| {
            for &scale in &FRAGMENT_SCALES_SMALL {
                let x = wp.x + (ss.rand.zero_one() - 0.5) * 8.0;
                let y = wp.y + (ss.rand.zero_one() * 5.0) + 5.0;
                let z = wp.z + (ss.rand.zero_one() - 0.5) * 8.0;
                let pos = Vec3::new(x, y, z);
                let mut velocity = bounced;
                velocity.x += (ss.rand.zero_one() - 0.5) * 11.0;
                velocity.y += ss.rand.zero_one() * 6.0;
                velocity.z += (ss.rand.zero_one() - 0.5) * 11.0;
                let phi_v0 = if ss.rand.zero_one() < 0.5 { 65 } else { 33 };
                ss.kakera_spawn(pos, velocity, pos, -420, phi_v0, 30, 5, 0, scale, 3, 10, 40, KAKERA_COLOR_NONE, OBJECT_GAMEPLAY_FIELD_KEEP, Some(("gameplay_field_keep", "gFieldKakeraDL")));
            }
        });
    }

    /// `EnIshi_SpawnFragmentsLarge`: nine pieces of `gSilverRockFragmentsDL` (`scales`), white,
    /// round the rock a turn of 0x4E20 apart from 0x1000 + 0x4E20 (up to 10 out, 5 to 45 up),
    /// with its bounced velocity plus up to 10 out and some up; the first big and heavy, the
    /// next three medium, the rest light; five bounces, 70 frames.
    fn spawn_fragments_large(&self, play: &mut PlayState) {
        let wp = self.actor.world_pos;
        let bounced = self.bounced_velocity(0.9, 0.8);
        play.with_ss(|ss| {
            let mut angle: i16 = 0x1000;
            for (i, &scale) in FRAGMENT_SCALES_LARGE.iter().enumerate() {
                angle = angle.wrapping_add(0x4E20);
                let rand = ss.rand.zero_one() * 10.0;
                let x = wp.x + (eng_math::sin_s(angle) * rand);
                let y = wp.y + (ss.rand.zero_one() * 40.0) + 5.0;
                let z = wp.z + (eng_math::cos_s(angle) * rand);
                let pos = Vec3::new(x, y, z);
                let mut velocity = bounced;
                let rand = ss.rand.zero_one() * 10.0;
                velocity.x += rand * eng_math::sin_s(angle);
                // (Rand_ZeroOne() * 4.0f) + ((Rand_ZeroOne() * i) * 0.7f): left to right, as IDO
                // evaluates the operands.
                let r1 = ss.rand.zero_one() * 4.0;
                let r2 = (ss.rand.zero_one() * i as f32) * 0.7;
                velocity.y += r1 + r2;
                velocity.z += rand * eng_math::cos_s(angle);
                let (phi_v0, phi_v1) = if i == 0 {
                    (41, -450)
                } else if i < 4 {
                    (37, -380)
                } else {
                    (69, -320)
                };
                ss.kakera_spawn(pos, velocity, wp, phi_v1, phi_v0, 30, 5, 0, scale, 5, 2, 70, KAKERA_COLOR_WHITE, OBJECT_GAMEPLAY_FIELD_KEEP, Some(("gameplay_field_keep", "gSilverRockFragmentsDL")));
            }
        });
    }

    /// The dust's centre: the rock's position, moved by twice its velocity off the ground or a
    /// wall.
    fn dust_pos(&self) -> Vec3 {
        let mut pos = self.actor.world_pos;
        let v = self.actor.velocity;
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND != 0 {
            pos.x += 2.0 * v.x;
            pos.y -= 2.0 * v.y;
            pos.z += 2.0 * v.z;
        } else if self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 {
            pos.x -= 2.0 * v.x;
            pos.y += 2.0 * v.y;
            pos.z -= 2.0 * v.z;
        }
        pos
    }

    /// `EnIshi_SpawnDustSmall`: four puffs 60 across, 80 big (`func_80033480`, lit).
    fn spawn_dust_small(&self, play: &mut PlayState) {
        func_80033480(play, self.dust_pos(), 60.0, 3, 0x50, 0x3C, 1);
    }

    /// `EnIshi_SpawnDustLarge`: eleven puffs 140 across, 180 big.
    fn spawn_dust_large(&self, play: &mut PlayState) {
        func_80033480(play, self.dust_pos(), 140.0, 0xA, 0xB4, 0x5A, 1);
    }
}

/// `sBreakSfxIds`, `sBreakSfxDurations` (the C sizes both 0x2852, `NA_SE_EV_ROCK_BROKEN`'s id).
const BREAK_SFX_IDS: [u16; 2] = [oot_game::audio::sfx::NA_SE_EV_ROCK_BROKEN, oot_game::audio::sfx::NA_SE_EV_WALL_BROKEN];
const BREAK_SFX_DURATIONS: [u16; 2] = [20, 40];

/// `EnIshi_SnapToFloor` / `EnKusa_SnapToFloor`: the floor below (from 30 above), plus `y`; the
/// home position follows. False if there's no floor.
pub(crate) fn snap_to_floor(actor: &mut Actor, play: &PlayState, y: f32) -> bool {
    // BgCheck_EntityRaycastDown4.
    let (floor, _) = play.col.entity_raycast_down(actor.world_pos + Vec3::Y * 30.0);
    if floor > eng_collision::bgcheck::BGCHECK_Y_MIN {
        actor.world_pos.y = floor + y;
        actor.home_pos = actor.world_pos;
        true
    } else {
        log::debug!("failure attaching to ground: {} at {}", actor.id, actor.world_pos);
        false
    }
}

impl ActorImpl for EnIshi {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `EnIshi_Update`.
    fn update(&mut self, play: &mut PlayState) {
        match self.action {
            Action::Wait => self.wait(play),
        }
    }
    /// `EnIshi_Draw`: `EnIshi_DrawSmall` (`Gfx_DrawDListOpa`) or `EnIshi_DrawLarge` (the
    /// same with prim 255, which `Gfx_SetupDL_25Opa` leaves).
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        let dl = if self.ty() == ROCK_SMALL { "gFieldKakeraDL" } else { "gSilverRockDL" };
        crate::gfx_draw_dlist_opa(out, "gameplay_field_keep", dl, rs);
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
