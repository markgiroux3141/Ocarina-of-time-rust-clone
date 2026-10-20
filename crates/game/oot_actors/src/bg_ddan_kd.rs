//! `Bg_Ddan_Kd` (`ovl_Bg_Ddan_Kd/z_bg_ddan_kd.c`): Dodongo's Cavern's falling stairs, a DynaPoly
//! actor of `object_ddan_objects` (`gDodongoFallingStairsCol`, `DYNA_TRANSFORM_POS`: it carries
//! Link) drawn with `gDodongoFallingStairsDL`.
//!
//! **What sets it off** (`BgDdanKd_CheckForExplosions`): two different explosions hitting its
//! cylinder (245 across, 180 high, from 400 below it: any hit but the shield's and the mirror's,
//! `AC_TYPE_ALL`), the second within 13 frames of the first and more than 80 from where the first
//! was (on x and z). Each explosive it catches gets `params` 2: its blast stops (`En_Bom` and
//! `En_Bombf` register the explosion's AT only with params 1) and it no longer sets off its
//! neighbours (`func_80033684` looks for params 1). Master Quest's room 2 lays two rows of Bomb
//! Flowers from the junction at z -1305 down to the stairs' foot: lit at the junction, both rows
//! burn down together and their last flowers blow by the stairs, either side, at once. One
//! explosive the C keeps for 13 frames (`timer`), then forgets (the next frame with none).
//!
//! **The fall** (`BgDdanKd_LowerStairs`): one-point cutscene 3050 (999 frames: the spline's own
//! length), the speed up 0.025 a frame towards 4, the stairs down to 220 below their home
//! (`Math_SmoothStepToF` at 0.075 of the way, at most the speed, at least 0.0075); every other
//! frame dust (`func_80033480`) and sparkles (`func_8003555C`) either side of the stairs (80 to 90
//! out on x, 210 to 440 or 330 to 570 along z by the frame) and one more at the foot (560 along z),
//! their size by how fast they fall; a quake on the main camera by the same strength, and
//! `NA_SE_EV_PILLAR_SINK`. Down, its switch flag set. With the flag set at init it starts down.
//!
//! The whole overlay is ported. Not ported: `Rumble_Request` (BACKLOG #60, logged). The dust's
//! and sparkles' z offsets are added in world space whatever the stairs' yaw, as in the C (room 2's
//! stairs face -x and run along +z), and their height is 20 over the stairs' `floorHeight`, which
//! nothing sets for a BG actor: 0, room 2's floor.
//!
//! Params: `switchFlag` the whole params (`Flags_GetSwitch(play, params)`: set, the stairs down).
//! Master Quest's room 2's is 0x000B.
//! Flags: reads and sets switch flag `switchFlag` (set when the stairs are down).

use eng_collision::dyna::{BG_ACTOR_MAX, BgActorSource, DYNA_TRANSFORM_POS};
use eng_collision::math3d::Cylinder16;
use eng_math::smooth_step_to_f;
use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor};
use oot_game::actor_ctx::{ACTORCAT_BG, ActorHandle, ActorImpl, ActorProfile, actor_get_collided_explosive, func_80033480};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind};
use oot_game::audio::sfx::{SFX_FLAG, SfxPos};
use oot_game::camera::CAM_ID_MAIN;
use oot_game::collision_check::*;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};

/// `ACTOR_BG_DDAN_KD` (`actor_table.h`: 0x005C).
pub const ACTOR_BG_DDAN_KD: i16 = 0x005C;
/// `OBJECT_DDAN_OBJECTS` (`object_table.h`: 0x002B).
pub const OBJECT: &str = "object_ddan_objects";
/// `gDodongoFallingStairsCol`, `gDodongoFallingStairsDL` (`object_ddan_objects.xml`).
pub const COLLISION: &str = "gDodongoFallingStairsCol";
pub const DL: &str = "gDodongoFallingStairsDL";

/// `NA_SE_EV_PILLAR_SINK` (`environmentbank_table.h`: 0x2827).
pub const NA_SE_EV_PILLAR_SINK: u16 = 0x2827;

/// The one-point cutscene of the stairs falling (`OnePointCutscene_Init(play, 3050, 999, ..)`).
pub const ONEPOINT_FALL: i16 = 3050;

/// How far the stairs fall (`home.pos.y - 200.0f - 20.0f`).
pub const FALL: f32 = 200.0 + 20.0;

/// `Bg_Ddan_Kd_Profile`: `ACTORCAT_BG`, `ACTOR_FLAG_UPDATE_CULLING_DISABLED`,
/// `OBJECT_DDAN_OBJECTS`.
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_BG_DDAN_KD,
    name: "Bg_Ddan_Kd",
    category: ACTORCAT_BG,
    flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED,
    object: OBJECT,
    schema: &SCHEMA,
};

/// `this->dyna.actor.params` whole, read as a switch flag.
pub const SWITCH_FLAG: Field = Field::whole("switchFlag", Kind::SwitchFlag, "set when the stairs are down; set at init, they start down");

/// Bg_Ddan_Kd's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[SWITCH_FLAG],
    flags: &[FlagUse::new(Flag::Switch("switchFlag")).reads().sets().note("set when two explosions have dropped the stairs (one-point 3050); set at init, they're down")],
    todo: false,
    objects: &[],
    texts: &[],
};

/// `sCylinderInit`: hit by anything but the shield and the mirror's ray (`0xFFCFFFFF`, any
/// attacker: `AC_TYPE_ALL`), 245 across, 180 high, 400 down.
pub const CYLINDER_INIT: ColliderCylinderInit = ColliderCylinderInit {
    base: ColliderInit { col_type: COL_MATERIAL_NONE, at_flags: AT_NONE, ac_flags: AC_ON | AC_TYPE_ALL, oc_flags1: OC1_NONE, oc_flags2: OC2_NONE, shape: COLSHAPE_CYLINDER },
    info: ColliderElementInit {
        elem_material: ELEM_MATERIAL_UNK2,
        at_dmg_info: ColliderElementDamageInfoAT { dmg_flags: 0x0000_0000, hit_special_effect: HIT_SPECIAL_EFFECT_NONE, damage: 0x00 },
        ac_dmg_info: ColliderElementDamageInfoACInit { dmg_flags: 0xFFCF_FFFF, hit_backlash: HIT_BACKLASH_NONE, defense: 0x00 },
        at_elem_flags: ATELEM_NONE,
        ac_elem_flags: ACELEM_ON,
        oc_elem_flags: OCELEM_NONE,
    },
    dim: Cylinder16 { radius: 245, height: 180, y_shift: -400, pos: [0, 0, 0] },
};

/// The collider's id (`collider_mut`).
pub const COL_CYLINDER: u8 = 0;

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `BgDdanKd_CheckForExplosions`.
    CheckForExplosions,
    /// `BgDdanKd_LowerStairs`.
    LowerStairs,
    /// `BgDdanKd_DoNothing`.
    DoNothing,
}

pub struct BgDdanKd {
    /// `dyna.actor`, `dyna.bgId`.
    pub actor: Actor,
    pub bg: u16,
    pub collider: ColliderCylinder,
    pub action: Action,
    /// `prevExplosive`, `prevExplosivePos`: the explosive kept, and where it was.
    pub prev_explosive: Option<ActorHandle>,
    pub prev_explosive_pos: Vec3,
    /// `timer`: the frames the kept explosive is kept for.
    pub timer: i16,
}

fn source(a: &Actor) -> BgActorSource {
    let r = a.shape_rot;
    BgActorSource { pos: a.world_pos, shape_rot: [r.x, r.y, r.z], scale: a.scale, shape_y_offset: a.shape_y_offset }
}

/// `Math_Vec3f_DistXZ`.
fn dist_xz(a: Vec3, b: Vec3) -> f32 {
    let (dx, dz) = (b.x - a.x, b.z - a.z);
    (dx * dx + dz * dz).sqrt()
}

impl BgDdanKd {
    /// `BgDdanKd_Init`: no explosive kept; scale 0.1 and the culling volume (`sInitChain`), the
    /// collision (`DYNA_TRANSFORM_POS`), the cylinder; waiting for explosions, or with its flag
    /// set down already (220 below its home), doing nothing.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // sInitChain: ICHAIN_VEC3F_DIV1000(scale, 100), cullingVolumeScale, cullingVolumeDownward
        // and cullingVolumeDistance 32767.
        actor.scale = Vec3::splat(0.1);
        actor.culling_volume_scale = 32767.0;
        actor.culling_volume_downward = 32767.0;
        actor.culling_volume_distance = 32767.0;
        let bg = match crate::obj_kibako2::load_collision(play, OBJECT, COLLISION) {
            Some(h) => play.col.dyna.set_bg_actor(h, source(&actor), DYNA_TRANSFORM_POS),
            None => BG_ACTOR_MAX,
        };
        let mut this = BgDdanKd { actor, bg, collider: ColliderCylinder::new(&CYLINDER_INIT), action: Action::CheckForExplosions, prev_explosive: None, prev_explosive_pos: Vec3::ZERO, timer: 0 };
        if !play.flags.get_switch(this.switch_flag() as i32) {
            this.action = Action::CheckForExplosions;
        } else {
            this.actor.world_pos.y = this.actor.home_pos.y - FALL;
            this.action = Action::DoNothing;
        }
        play.col.dyna.set_source(this.bg, source(&this.actor));
        Box::new(this)
    }

    /// `this->dyna.actor.params`: the switch flag.
    pub fn switch_flag(&self) -> i16 {
        SWITCH_FLAG.get(self.actor.params)
    }

    /// `BgDdanKd_CheckForExplosions`: an explosive's hit gets its `params` 2; a second, different
    /// one more than 80 from the first (on x and z) while the first is kept, the fall and
    /// one-point 3050 (999). Else the kept one counts down 13 frames, and with the count out the
    /// frame's explosive (or none) is kept, an explosive with 13 again and its place; the cylinder
    /// placed and registered.
    fn check_for_explosions(&mut self, play: &mut PlayState) {
        let explosive = actor_get_collided_explosive(play, &mut self.collider.base);
        if let Some(e) = explosive {
            log::debug!("Bg_Ddan_Kd: dam    {}", self.actor.col_chk_info.damage);
            if let Some(a) = play.actors.actor_mut(e) {
                a.params = 2;
            }
        }
        let explosive_pos = explosive.and_then(|e| play.actors.actor(e)).map(|a| a.world_pos);
        if let (Some(e), Some(prev), Some(pos)) = (explosive, self.prev_explosive, explosive_pos)
            && e != prev
            && dist_xz(self.prev_explosive_pos, pos) > 80.0
        {
            self.action = Action::LowerStairs;
            let me = play.cur_actor.map(|h| play.cam_actor_of(h, &self.actor));
            play.onepoint_cutscene_init(ONEPOINT_FALL, 999, me, CAM_ID_MAIN);
        } else {
            if self.timer != 0 {
                self.timer -= 1;
            } else {
                self.prev_explosive = explosive;
                if let Some(pos) = explosive_pos {
                    self.timer = 13;
                    self.prev_explosive_pos = pos;
                }
            }
            self.collider.update(&self.actor);
            play.collision_check_set_ac(&self.actor, COL_CYLINDER, &mut self.collider);
        }
    }

    /// `BgDdanKd_LowerStairs`: the speed towards 4, the rumble, the stairs down towards 220 below
    /// their home; down, the flag set, doing nothing; else on odd `state.frames` the dust and
    /// sparkles either side and at the foot (their size `effectStrength`: the fall this frame plus
    /// a quarter of the speed), the main camera's quake, the sinking sound.
    fn lower_stairs(&mut self, play: &mut PlayState) {
        // static Vec3f velocity = { 0.0f, 5.0f, 0.0f }: its x and z written every time (y stays 5).
        let mut velocity = Vec3::new(0.0, 5.0, 0.0);
        let accel = Vec3::new(0.0, -0.45, 0.0);
        smooth_step_to_f(&mut self.actor.speed_xz, 4.0, 0.5, 0.025, 0.0);
        // Rumble_Request(500.0f, 120, 20, 10): no rumble (BACKLOG #60).
        log::trace!("Bg_Ddan_Kd: Rumble_Request(500, 120, 20, 10) not ported");
        let target = self.actor.home_pos.y - FALL;
        let speed = self.actor.speed_xz;
        if smooth_step_to_f(&mut self.actor.world_pos.y, target, 0.075, speed, 0.0075) == 0.0 {
            play.flags.set_switch(self.switch_flag() as i32);
            self.action = Action::DoNothing;
            return;
        }
        let effect_strength = (self.actor.prev_pos.y - self.actor.world_pos.y) + (self.actor.speed_xz * 0.25);
        if play.state_frames & 1 != 0 {
            let p = self.actor.world_pos;
            let floor = self.actor.floor_height;
            let (mut pos1, mut pos2) = (p, p);
            let r = &mut play.rand;
            if play.state_frames & 2 != 0 {
                pos1.z += 210.0 + r.zero_one() * 230.0;
                pos2.z += 210.0 + r.zero_one() * 230.0;
            } else {
                pos1.z += 330.0 + r.zero_one() * 240.0;
                pos2.z += 330.0 + r.zero_one() * 240.0;
            }
            pos1.x += 80.0 + r.zero_one() * 10.0;
            pos2.x -= 80.0 + r.zero_one() * 10.0;
            pos1.y = floor + 20.0 + r.zero_one();
            pos2.y = floor + 20.0 + r.zero_one();
            let scale = (effect_strength * 135.0) as i16;
            func_80033480(play, pos1, 20.0, 1, scale, 60, 1);
            func_80033480(play, pos2, 20.0, 1, scale, 60, 1);
            velocity.x = play.rand.centered_float(3.0);
            velocity.z = play.rand.centered_float(3.0);
            func_8003555c(play, pos1, velocity, accel);
            func_8003555c(play, pos2, velocity, accel);
            let mut pos1 = p;
            let r = &mut play.rand;
            pos1.z += 560.0 + r.zero_one() * 5.0;
            pos1.x += (r.zero_one() - 0.5) * 160.0;
            pos1.y = r.zero_one() * 3.0 + (floor + 20.0);
            func_80033480(play, pos1, 20.0, 1, scale, 60, 1);
            func_8003555c(play, pos1, velocity, accel);
        }
        play.camera_request_quake(CAM_ID_MAIN, (effect_strength * 0.6) as i16, 3);
        if let Some(me) = play.cur_actor {
            // SFX_PLAY_AT_POS(&this->dyna.actor.projectedPos, NA_SE_EV_PILLAR_SINK - SFX_FLAG).
            play.audio.play_sfx_at_pos(SfxPos::Actor(me), NA_SE_EV_PILLAR_SINK - SFX_FLAG);
        }
    }
}

// shared: func_8003555C
/// `func_8003555C` (`z_actor.c`): a small sparkle (`EffectSsKiraKira_SpawnSmall`) at `pos` with
/// the given velocity and acceleration, prim (200, 160, 120) and env (130, 90, 50). @bug (game):
/// the colours' alphas aren't set (stack garbage in the C, 0 here).
fn func_8003555c(play: &mut PlayState, pos: Vec3, velocity: Vec3, accel: Vec3) {
    play.with_ss(|ss| ss.kirakira_spawn_small(pos, velocity, accel, [200, 160, 120, 0], [130, 90, 50, 0]));
}

impl ActorImpl for BgDdanKd {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `BgDdanKd_Update`: the action; the new transform for `DynaPoly_UpdateContext`.
    fn update(&mut self, play: &mut PlayState) {
        match self.action {
            Action::CheckForExplosions => self.check_for_explosions(play),
            Action::LowerStairs => self.lower_stairs(play),
            Action::DoNothing => {}
        }
        play.col.dyna.set_source(self.bg, source(&self.actor));
    }

    /// `BgDdanKd_Destroy`: the collision and the cylinder.
    fn destroy(&mut self, play: &mut PlayState) {
        play.col.dyna.delete_bg_actor(self.bg);
    }

    fn dyna_bg_id(&self) -> Option<u16> {
        Some(self.bg)
    }

    /// `BgDdanKd_Draw`: `Gfx_DrawDListOpa(gDodongoFallingStairsDL)`.
    fn draw(&self, rs: &RenderState, _play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        crate::gfx_draw_dlist_opa(out, OBJECT, DL, rs);
    }

    fn collider_mut(&mut self, id: u8) -> Option<ColliderMut<'_>> {
        match id {
            COL_CYLINDER => Some(ColliderMut::Cylinder(&mut self.collider)),
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
