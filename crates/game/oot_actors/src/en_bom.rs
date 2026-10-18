//! `En_Bom` (`ovl_En_Bom/z_en_bom.c`): a bomb, and its explosion. Link takes one out of his bomb
//! bag in his hand (`Player_InitExplosiveIA`), and enemies spawn them (a Peahat's death, the
//! courtyard guard's throw, a Dodongo's, an Armos's, a Beamos's).
//!
//! It appears three frames on (`NA_SE_PL_TAKE_OUT_SHIELD`, scale 0.01) with a fuse of 70 frames:
//! from 63 its sparks (`Effect_Ss_G_Spk`) and smoke, its hiss, and its red flash quickening at 40,
//! 20 and 3. Held it rides in Link's hand (no room); let go it falls, slows, rebounds off walls and
//! bounces off the ground, and lying there (fuse 4 and up) it offers itself to be picked up again.
//! A hit (an explosion, an arrow, Din's Fire) or an enemy touching it sets it off at once; deep
//! water (20 under) puts it out (`NA_SE_IT_BOMB_UNEXPLOSION`).
//!
//! At 0 it explodes: the blast (`Effect_Ss_Bomb2`, layered), a white shockwave on the floor
//! (`Effect_Ss_Blast`), `NA_SE_IT_BOMB_EXPLOSION`, the light flash (`envCtx.adjLight1Color` and
//! `adjAmbientColor` 250, fading 25 a frame), a quake on the main camera; then for ten frames a
//! sphere of `DMG_EXPLOSIVE` (damage 8 and `shape.rot.z`'s high byte) growing by 8 and the low
//! byte a frame, centred where the bomb last drew. Gone in Link's hand, it lets go of him itself.
//!
//! The circle shadow (16) is the shape's (`ActorShape_Init`'s, drawn by `Actor_Draw`).
//!
//! Not ported: `Rumble_Request` (BACKLOG #60), the shine (`func_8002EBCC`'s look-at: the
//! renderer's texgen reads the view).
//!
//! Params: `type` (`EnBomType`: 0 `BOMB_BODY`, the bomb; 1 `BOMB_EXPLOSION` once it blows, which
//! `func_80033684` and the actors that check for a blast read). `shape.rot.z`'s high byte
//! (`damageBonus`, added to the explosion's damage) and low byte (`growth`, signed: added to the
//! sphere's growth each frame and, times 6, to the blast's scale step); the spawners set it: the
//! Peahat 0x602, the Beamos 0x6FF and 0x601, the Dodongo 6, the Armos 2.
//! Flags: none.

use glam::{Mat4, Vec3};
use oot_game::actor::{
    ACTOR_FLAG_DRAW_CULLING_DISABLED, ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor, BGCHECKFLAG_CEILING, BGCHECKFLAG_GROUND, BGCHECKFLAG_GROUND_TOUCH, BGCHECKFLAG_WALL, BGCHECKFLAG_WATER_TOUCH,
    UPDBGCHECKINFO_FLAG_0, UPDBGCHECKINFO_FLAG_1, UPDBGCHECKINFO_FLAG_2, UPDBGCHECKINFO_FLAG_3, UPDBGCHECKINFO_FLAG_4,
};
use oot_game::actor_ctx::{ACTORCAT_ENEMY, ACTORCAT_EXPLOSIVE, ActorImpl, ActorProfile, actor_play_sfx_surface_bomb, audio_play_actor_sfx2};
use oot_game::actor_schema::{ActorSchema, Field, Kind, Source};
use oot_game::audio::sfx::{NA_SE_EV_BOMB_BOUND, NA_SE_EV_BOMB_DROP_WATER, NA_SE_IT_BOMB_EXPLOSION, NA_SE_IT_BOMB_IGNIT, NA_SE_IT_BOMB_UNEXPLOSION, NA_SE_PL_TAKE_OUT_SHIELD, SFX_FLAG};
use oot_game::collision_check::*;
use oot_game::effect::SsActor;
use oot_game::effect::dead_sound::DEADSOUND_REPEAT_MODE_OFF;
use oot_game::pack::{BakeBody, BakeSegment, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};

pub const ACTOR_EN_BOM: i16 = 0x0010;

/// `PLAYER_STATE1_CARRYING_ACTOR` (`player.h`: 1 << 11).
const PLAYER_STATE1_CARRYING_ACTOR: u32 = 1 << 11;

/// `En_Bom_Profile`.
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_EN_BOM,
    name: "En_Bom",
    category: ACTORCAT_EXPLOSIVE,
    flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_DRAW_CULLING_DISABLED,
    object: "gameplay_keep",
    schema: &SCHEMA,
};

/// `EnBomType`: the whole params.
pub const TYPE: Field = Field::whole("type", Kind::Named(&[(0, "BOMB_BODY"), (1, "BOMB_EXPLOSION")]), "a bomb (spawned so); an explosion once it blows");
/// `(shape.rot.z & 0xFF00) >> 8`: added to the explosion's damage.
pub const DAMAGE_BONUS: Field = Field::rot("damageBonus", Source::RotZ, 8, 8, Kind::Range(0, 255), "added to the explosion's damage (8)");
/// `shape.rot.z & 0xFF`, sign-extended: the sphere's extra growth a frame.
pub const GROWTH: Field = Field::rot("growth", Source::RotZ, 0, 8, Kind::Range(-128, 127), "added to the explosion sphere's growth (8 a frame), and ×6 to the blast's scale step").signed();

/// En_Bom's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema { fields: &[TYPE, DAMAGE_BONUS, GROWTH], flags: &[], todo: false, objects: &[] };

/// `EnBomType`.
const BOMB_BODY: i16 = 0;
const BOMB_EXPLOSION: i16 = 1;

/// `sCylinderInit`: what sets it off (`0x0003F828`: an explosion, arrows, Din's Fire).
const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_HIT0, at_flags: AT_NONE, ac_flags: AC_ON | AC_TYPE_PLAYER | AC_TYPE_OTHER, oc_flags1: OC1_ON | OC1_TYPE_ALL, oc_flags2: OC2_TYPE_2, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK2,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0x0003_F828, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_ON,
    },
    dim: eng_collision::math3d::Cylinder16 { radius: 6, height: 11, y_shift: 14, pos: [0; 3] },
};

/// `sJntSphElementsInit`: the explosion, `DMG_EXPLOSIVE` damage 8, its model sphere 0 across.
const JNT_SPH_ELEMENTS: [ColliderJntSphElementInit; 1] = [ColliderJntSphElementInit {
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK0,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0x0000_0008, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x08 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0, hit_backlash: HIT_BACKLASH_NONE, defense: 0 },
        at_elem_flags: ATELEM_ON | ATELEM_SFX_NONE,
        ac_elem_flags: ACELEM_NONE,
        oc_elem_flags: OCELEM_NONE,
    },
    limb: 0,
    model_sphere: eng_collision::math3d::Sphere16 { center: [0, 0, 0], radius: 0 },
    scale: 100,
}];

/// `sJntSphInit`.
const JNT_SPH_INIT: ColliderInit = ColliderInit { col_type: COL_MATERIAL_HIT0, at_flags: AT_ON | AT_TYPE_ALL, ac_flags: AC_NONE, oc_flags1: OC1_NONE, oc_flags2: OC2_NONE, shape: COLSHAPE_JNTSPH };

/// The colliders' ids (`collider_mut`).
const COL_BOMB: u8 = 0;
const COL_EXPLOSION: u8 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    /// `EnBom_Move`.
    Move,
    /// `EnBom_WaitForRelease`.
    WaitForRelease,
    /// `EnBom_Explode`.
    Explode,
}

pub struct EnBom {
    pub actor: Actor,
    pub bomb_collider: ColliderCylinder,
    pub explosion_collider: ColliderJntSph,
    /// `timer`: the fuse, then the explosion's ten frames. Spawners that want it to blow at once
    /// set it to 0 (the Peahat's death).
    pub timer: i16,
    pub flash_speed_scale: i16,
    pub flash_intensity: f32,
    pub collider_set_oc: bool,
    action: Action,
}

impl EnBom {
    /// `EnBom_Init`: unseen (scale 0), the target arrow 2000 up, gravity -4; 700 up its model; mass
    /// 200, its body 5 across and 10 high; a fuse of 70; the explosion's damage raised by
    /// `shape.rot.z`'s high byte, and `shape.rot.z` left as its low byte, sign-extended.
    pub fn init(mut actor: Actor, _play: &mut PlayState) -> Box<dyn ActorImpl> {
        // Actor_ProcessInitChain: scale 0, lockOnArrowOffset 2000, gravity -4.
        actor.scale = Vec3::ZERO;
        actor.target_arrow_offset = 2000.0;
        actor.gravity = -4.0;
        actor.shape_init(700.0, oot_game::shadow::ShadowDraw::Circle, 16.0);
        actor.col_chk_info.mass = 200;
        actor.col_chk_info.cyl_radius = 5;
        actor.col_chk_info.cyl_height = 10;
        let bomb_collider = ColliderCylinder::new(&CYLINDER_INIT);
        let mut explosion_collider = ColliderJntSph::new(&JNT_SPH_INIT, &JNT_SPH_ELEMENTS);
        let bonus = DAMAGE_BONUS.get(actor.shape_rot.z) as u8;
        explosion_collider.elements[0].info.at_dmg_info.damage = explosion_collider.elements[0].info.at_dmg_info.damage.wrapping_add(bonus);
        actor.shape_rot.z = GROWTH.get(actor.shape_rot.z);
        Box::new(EnBom { actor, bomb_collider, explosion_collider, timer: 70, flash_speed_scale: 7, flash_intensity: 0.0, collider_set_oc: false, action: Action::Move })
    }

    fn ty(&self) -> i16 {
        TYPE.get(self.actor.params)
    }

    /// `Actor_SetScale`.
    fn set_scale(&mut self, scale: f32) {
        self.actor.scale = Vec3::splat(scale);
    }

    /// `EnBom_Move`: taken up (a parent) it waits to be let go, in no room. Otherwise: rising into a
    /// ceiling turns it down; moving into a wall it rebounds (the yaw mirrored off the wall's when
    /// it hit more than square on), with `NA_SE_EV_BOMB_BOUND` and a step, at 0.7 the speed; it
    /// slows (by 0.08 in the air, 1 on the ground); landing faster than 3 it bounces (`vy × -0.3`,
    /// the surface's sound), else on the ground with 4 or more on the fuse it offers itself to be
    /// picked up; then it moves (`Actor_MoveXZGravity`).
    fn move_(&mut self, play: &mut PlayState) {
        if self.actor.parent.is_some() {
            self.action = Action::WaitForRelease;
            self.actor.room = -1;
            return;
        }
        if self.actor.velocity.y > 0.0 && self.actor.bg_check_flags & BGCHECKFLAG_CEILING != 0 {
            self.actor.velocity.y = -self.actor.velocity.y;
        }
        if self.actor.speed_xz != 0.0 && self.actor.bg_check_flags & BGCHECKFLAG_WALL != 0 {
            let yaw_diff = self.actor.wall_yaw.wrapping_sub(self.actor.world_rot.y);
            if (yaw_diff as i32).abs() > 0x4000 {
                self.actor.world_rot.y = self.actor.wall_yaw.wrapping_sub(self.actor.world_rot.y).wrapping_add(self.actor.wall_yaw).wrapping_sub(0x8000u16 as i16);
            }
            audio_play_actor_sfx2(play, NA_SE_EV_BOMB_BOUND);
            self.actor.move_forward();
            self.actor.speed_xz *= 0.7;
            self.actor.bg_check_flags &= !BGCHECKFLAG_WALL;
        }
        if self.actor.bg_check_flags & BGCHECKFLAG_GROUND == 0 {
            eng_math::step_to_f(&mut self.actor.speed_xz, 0.0, 0.08);
        } else {
            eng_math::step_to_f(&mut self.actor.speed_xz, 0.0, 1.0);
            if self.actor.bg_check_flags & BGCHECKFLAG_GROUND_TOUCH != 0 && self.actor.velocity.y < -3.0 {
                actor_play_sfx_surface_bomb(play, &self.actor);
                self.actor.velocity.y *= -0.3;
                self.actor.bg_check_flags &= !BGCHECKFLAG_GROUND_TOUCH;
            } else if self.timer >= 4 {
                let a = self.actor.clone();
                oot_game::get_item::offer_pick_up(play, &a);
            }
        }
        self.actor.move_forward();
    }

    /// `EnBom_WaitForRelease`: let go, it moves on this frame.
    fn wait_for_release(&mut self, play: &mut PlayState) {
        if self.actor.parent.is_none() {
            self.action = Action::Move;
            self.move_(play);
        }
    }

    /// `EnBom_Explode`: the sphere grows (`rot.z + 8`, with draw culling off; the rumble isn't
    /// ported), its hits registered while it's an explosion; the light flash fades 25 a frame; at
    /// 0 it lets go of Link if he holds it, and is gone.
    fn explode(&mut self, play: &mut PlayState) {
        if self.explosion_collider.elements[0].dim.model_sphere.radius == 0 {
            self.actor.flags |= ACTOR_FLAG_DRAW_CULLING_DISABLED;
            // Rumble_Request(xzDistToPlayer, 255, 20, 150): not ported (BACKLOG #60).
        }
        let ws = &mut self.explosion_collider.elements[0].dim.world_sphere;
        ws.radius = ws.radius.wrapping_add(self.actor.shape_rot.z.wrapping_add(8));
        if self.ty() == BOMB_EXPLOSION {
            play.collision_check_set_at(&self.actor, COL_EXPLOSION, &mut self.explosion_collider);
        }
        fade_explosion_light(play);
        if self.timer == 0 {
            let me = play.cur_actor;
            release_from_player(play, me);
            self.actor.kill();
        }
    }
}

/// The light flash's fade (`EnBom_Explode`, `EnBombf_Explode`): each channel of
/// `envCtx.adjLight1Color` and `adjAmbientColor` not at 0 down by 25.
pub(crate) fn fade_explosion_light(play: &mut PlayState) {
    for c in play.env_ctx.adj_light1_color.iter_mut().chain(play.env_ctx.adj_ambient_color.iter_mut()) {
        if *c != 0 {
            *c -= 25;
        }
    }
}

/// The explosion's flash (`EnBom_Update`, `EnBombf_Update`): `envCtx.adjLight1Color` and
/// `adjAmbientColor` all 250.
pub(crate) fn flash_explosion_light(play: &mut PlayState) {
    play.env_ctx.adj_light1_color = [250; 3];
    play.env_ctx.adj_ambient_color = [250; 3];
}

/// An explosive gone in Link's hand (`EnBom_Explode`, `EnBombf_Explode`): if he's carrying it
/// (`PLAYER_STATE1_CARRYING_ACTOR`, `heldActor == this`), his `actor.child`, `heldActor` and
/// `interactRangeActor` cleared and the carrying flag off.
pub(crate) fn release_from_player(play: &mut PlayState, me: Option<oot_game::actor_ctx::ActorHandle>) {
    let Some(pi) = play.player.and_then(|h| play.actors.get_mut(h)).and_then(|p| p.as_player_mut()) else { return };
    if pi.state_flags1() & PLAYER_STATE1_CARRYING_ACTOR != 0 && me.is_some() && pi.held_actor() == me {
        pi.clear_carried();
    }
}

/// The updating explosive as `initParams->actor` for its fuse's spark.
pub(crate) fn ss_actor(play: &PlayState, actor: &Actor) -> Option<SsActor> {
    play.cur_actor.map(|handle| SsActor { handle, world_pos: actor.world_pos, shape_rot: [actor.shape_rot.x, actor.shape_rot.y, actor.shape_rot.z] })
}

/// The bake names: the cap, and the body with its colours dynamic.
const BAKE_CAP: &str = "En_Bom/cap";
const BAKE_BODY: &str = "En_Bom/body";
/// The body's colours' segment (`gDPSetEnvColor`, `gDPSetPrimColor` before `gBombBodyDL`).
const SEG_COLOR: u8 = 0x0B;

/// `EnBom_Draw`'s lists after `Gfx_SetupDL_25Opa`: `gBombCapDL`, and `gBombBodyDL` with its prim and
/// env colours dynamic.
pub fn bakes() -> Vec<MeshBake> {
    vec![
        MeshBake { name: BAKE_CAP.into(), object: "gameplay_keep".into(), segments: vec![], prelude: vec![], body: BakeBody::DLists(vec![("gameplay_keep".into(), "gBombCapDL".into())]) },
        MeshBake {
            name: BAKE_BODY.into(),
            object: "gameplay_keep".into(),
            segments: vec![(SEG_COLOR, BakeSegment::DynamicColor { env: true, prim: true })],
            prelude: vec![SEG_COLOR],
            body: BakeBody::DLists(vec![("gameplay_keep".into(), "gBombBodyDL".into())]),
        },
    ]
}

/// `rs.values`: `flashIntensity`; `rs.switches`: whether it's a bomb (`BOMB_BODY`).
mod rs {
    pub const FLASH: usize = 0;
    pub const BODY: usize = 0;
}

impl ActorImpl for EnBom {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnBom_Update`.
    fn update(&mut self, play: &mut PlayState) {
        let eff_velocity = Vec3::ZERO;
        let bomb2_accel = Vec3::new(0.0, 0.1, 0.0);
        let eff_accel = Vec3::ZERO;
        let mut dust_accel = Vec3::new(0.0, 0.6, 0.0);
        let dust_color = [255, 255, 255, 255];
        self.actor.gravity = -1.2;
        if self.timer != 0 {
            self.timer -= 1;
        }
        if self.timer == 67 {
            audio_play_actor_sfx2(play, NA_SE_PL_TAKE_OUT_SHIELD);
            self.set_scale(0.01);
        }
        if self.actor.xz_dist_to_player >= 20.0 || self.actor.y_dist_to_player.abs() >= 80.0 {
            self.collider_set_oc = true;
        }
        match self.action {
            Action::Move => self.move_(play),
            Action::WaitForRelease => self.wait_for_release(play),
            Action::Explode => self.explode(play),
        }
        self.actor.update_bg_check_info(&play.col, 5.0, 10.0, 15.0, UPDBGCHECKINFO_FLAG_0 | UPDBGCHECKINFO_FLAG_1 | UPDBGCHECKINFO_FLAG_2 | UPDBGCHECKINFO_FLAG_3 | UPDBGCHECKINFO_FLAG_4);
        if self.ty() == BOMB_BODY {
            if self.timer < 63 {
                dust_accel.y = 0.2;
                // The spark on even frames, 17 up.
                let mut eff_pos = self.actor.world_pos;
                eff_pos.y += 17.0;
                if play.gameplay_frames % 2 == 0 {
                    let a = ss_actor(play, &self.actor);
                    play.with_ss(|s| s.g_spk_spawn_fuse(a, eff_pos, eff_velocity, eff_accel));
                }
                audio_play_actor_sfx2(play, NA_SE_IT_BOMB_IGNIT - SFX_FLAG);
                eff_pos.y += 3.0;
                play.with_ss(|s| s.func_8002829c(eff_pos, eff_velocity, dust_accel, dust_color, dust_color, 50, 5));
            }
            let enemy_touch = self.bomb_collider.base.oc_flags1 & OC1_HIT != 0 && self.bomb_collider.base.oc.and_then(|h| play.actors.actor(h)).is_some_and(|a| a.category == ACTORCAT_ENEMY);
            if self.bomb_collider.base.ac_flags & AC_HIT != 0 || enemy_touch {
                self.timer = 0;
                self.actor.shape_rot.z = 0;
            } else if self.timer > 100 && crate::player_is_burning_stick_in_range(play, self.actor.world_pos, 30.0, 50.0) {
                // These bombs never have a fuse over 70: unused.
                self.timer = 100;
            }
            // (dustAccel.y and effPos set again, unused.)
            if self.timer == 3 || self.timer == 20 || self.timer == 40 {
                self.actor.shape_rot.z = 0;
                self.flash_speed_scale >>= 1;
            }
            // 140 / flashSpeedScale: infinite once it's halved to 0, as the C's float division.
            let step = 140.0 / self.flash_speed_scale as f32;
            if self.timer < 100 && (self.timer & (self.flash_speed_scale + 1)) != 0 {
                eng_math::smooth_step_to_f(&mut self.flash_intensity, 140.0, 1.0, step, 0.0);
            } else {
                eng_math::smooth_step_to_f(&mut self.flash_intensity, 0.0, 1.0, step, 0.0);
            }
            if self.timer < 3 {
                self.set_scale(self.actor.scale.x + 0.002);
            }
            if self.timer == 0 {
                let mut eff_pos = self.actor.world_pos;
                eff_pos.y += 10.0;
                if self.actor.parent.is_some() {
                    eff_pos.y += 30.0;
                }
                let scale_step = (self.actor.shape_rot.z as i32 * 6 + 19) as i16;
                play.with_ss(|s| s.bomb2_spawn_layered(eff_pos, eff_velocity, bomb2_accel, 100, scale_step));
                eff_pos.y = self.actor.floor_height;
                if self.actor.floor_height > eng_collision::bgcheck::BGCHECK_Y_MIN {
                    play.with_ss(|s| s.blast_spawn_white_shockwave(eff_pos, eff_velocity, eff_accel));
                }
                audio_play_actor_sfx2(play, NA_SE_IT_BOMB_EXPLOSION);
                flash_explosion_light(play);
                play.camera_request_quake(oot_game::camera::CAM_ID_MAIN, 11, 8);
                self.actor.params = BOMB_EXPLOSION; // schema: the C's write, the bomb blowing
                self.timer = 10;
                self.actor.flags |= ACTOR_FLAG_DRAW_CULLING_DISABLED;
                self.action = Action::Explode;
            }
        }
        self.actor.set_focus(20.0);
        if self.ty() <= BOMB_BODY {
            self.bomb_collider.update(&self.actor);
            // Not while Link holds it, and not until he's 20 away (or 80 above or below) once.
            if self.actor.parent.is_none() && self.collider_set_oc {
                play.collision_check_set_oc(&self.actor, COL_BOMB, &mut self.bomb_collider);
            }
            play.collision_check_set_ac(&self.actor, COL_BOMB, &mut self.bomb_collider);
        }
        if self.actor.scale.x >= 0.01 && self.ty() != BOMB_EXPLOSION {
            if self.actor.y_dist_to_water >= 20.0 {
                let pos = self.actor.projected_pos;
                play.with_ss(|s| s.dead_sound_spawn_stationary(pos, NA_SE_IT_BOMB_UNEXPLOSION, 1, DEADSOUND_REPEAT_MODE_OFF, 10));
                self.actor.kill();
                return;
            }
            if self.actor.bg_check_flags & BGCHECKFLAG_WATER_TOUCH != 0 {
                self.actor.bg_check_flags &= !BGCHECKFLAG_WATER_TOUCH;
                audio_play_actor_sfx2(play, NA_SE_EV_BOMB_DROP_WATER);
            }
        }
    }

    /// `EnBom_Draw`'s `Collider_UpdateSpheres(0, &explosionCollider)` while it's a bomb: the
    /// sphere at the draw's matrix (its translation: the model sphere is at 0 and 0 across).
    fn draw_update(&mut self, _play: &mut PlayState) {
        if self.ty() == BOMB_BODY {
            let m = oot_game::play::actor_draw_matrix(&RenderState::of(&self.actor));
            self.explosion_collider.update_spheres(0, &m);
        }
    }

    fn render_state(&self) -> RenderState {
        let mut r = RenderState::of(&self.actor);
        r.values = vec![self.flash_intensity];
        r.switches = vec![(self.ty() == BOMB_BODY) as u32];
        r
    }

    /// `EnBom_Draw`: while it's a bomb, `gBombCapDL` turned to face the camera
    /// (`Matrix_ReplaceRotation(&play->billboardMtxF)`), then turned a quarter about x
    /// `gBombBodyDL` with the flash as env and prim `((s16)flashIntensity, 0, 40)`.
    fn draw(&self, rs: &RenderState, _play: &PlayState, view: &ViewInfo, out: &mut DrawOut) {
        if rs.switches.get(rs::BODY).copied().unwrap_or(0) == 0 {
            return;
        }
        let cap = oot_game::draw::replace_rotation(oot_game::play::actor_draw_matrix(rs), view.billboard);
        out.opa.push(eng_gfx::DrawCmd::new(eng_gfx::MeshKey::named(keys::bake(BAKE_CAP)), cap));
        let body = cap * Mat4::from_rotation_x(oot_game::sys_matrix::binang_to_rad(0x4000));
        let flash = rs.values.get(rs::FLASH).copied().unwrap_or(0.0) as i16 as u8;
        let mut sv = eng_gfx::SegmentValues::default();
        sv.env[SEG_COLOR as usize] = Some([flash, 0, 40, 255]);
        sv.prim[SEG_COLOR as usize] = Some([flash, 0, 40, 255]);
        out.opa.push(eng_gfx::DrawCmd { mesh: eng_gfx::MeshKey::named(keys::bake(BAKE_BODY)), transform: body, bones: Vec::new(), params: eng_gfx::DrawParams { segments: Some(sv), ..Default::default() } });
    }

    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        match id {
            COL_BOMB => Some(ColliderMut::Cylinder(&mut self.bomb_collider)),
            COL_EXPLOSION => Some(ColliderMut::JntSph(&mut self.explosion_collider)),
            _ => None,
        }
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
