//! `En_Encount2` (`ovl_En_Encount2/z_en_encount2.c`): the falling rocks' spawner (GAME-09
//! milestone 6), on Death Mountain Trail's summit and in Ganon's Tower's collapse.
//!
//! - **Death Mountain Trail** (`SCENE_DEATH_MOUNTAIN_TRAIL`; killed at init for an adult with
//!   `EVENTCHKINF_49`, the Fire Temple's blue warp): while Link is above y 1500 with x in
//!   (-700, 100) and z in (-3600, -1290) (`EnEncount2_Wait`), a quake (type 1, speed 0x7FFF, 50 up
//!   and down, 300 frames; again each round), and every 300 frames a round of 200
//!   (`EnEncount2_SpawnRocks`, its z bound -3860): the volcano's rumble (`NA_SE_EV_VOLCANO`), a
//!   falling ember every frame, and up to two `En_Fire_Rock` at once, 4 frames apart, 400 above
//!   the point 160 ahead of the camera (by its pitch). Half of a child's (`Rand_ZeroFloat(1.99) <
//!   1`) are `_FALLING1` within 5 of Link; the rest `_FALLING2`, within 5 of Link when he stands
//!   still, else ahead of the camera (x 300 along its facing ±100, z 600 along it ±25). The rocks count themselves
//!   off at their destroy. Its darkening: over 60 frames of the rocks the ambient and the light
//!   redden (`adjAmbientColor` (75, -50, -50), `adjLight1Color` (30, -20, -20) at full), the fog
//!   comes 50 nearer and goes to (160, 160, 150); it lifts while he's out of the area.
//! - **Ganon's Tower's collapse** (`SCENE_GANON_BOSS`, `_GANONS_TOWER_COLLAPSE_EXTERIOR`,
//!   `_INTERIOR`, `SCENE_INSIDE_GANONS_CASTLE_COLLAPSE`; placed in `ganon_final` only): with switch
//!   0x37 set, Link within 700 across, and `collapseSpawnerInactive` clear (`En_Zl3` sets it), the
//!   same rounds without the quake, the darkening or the rumble: an ember every 5 frames, one
//!   `_FALLING2` at a time 50 frames apart, 300 ahead of the camera (±50) or, a quarter of the
//!   time, within 35 of Link. Its embers go 50 below Link's floor (the trail's below y 1500).
//!
//! Its embers (`EnEncount2Effect`, 50): 20 down a frame (±10 across), tumbling at random; drawn
//! as the rock (`object_efc_star_field_DL_000DE0`, the same colours) at their own scale (0.0045 to
//! 0.0095) without a setup of their own (`Demo_Kankyo`'s rock bake, `Gfx_SetupDL_25Opa`'s, stands
//! in for the state the frame leaves).
//!
//! **@bug (game)**, kept: the fog colour's adjustment is cast to `u8` (`(160 - fogColor) × t`): a
//! fog redder or greener than 160 (bluer than 150) wraps round to a large one. As the C does, a
//! round under way keeps rumbling and darkening with Link out of the area (only the wait lifts it).
//!
//! The whole overlay is ported.
//!
//! Params: `params` printed at init, used by nothing (5 on the trail, -1 in `ganon_final`).
//! Flags: switch 0x37 (the tower's collapse begun) read; `EVENTCHKINF_49` read.

use eng_gfx::{DrawCmd, MeshKey};
use eng_math::approach_f;
use glam::{Mat4, Vec3};
use oot_game::actor::*;
use oot_game::actor_ctx::{ACTORCAT_ENEMY, ActorImpl, ActorProfile, audio_play_actor_sfx2};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind};
use oot_game::audio::sfx::{NA_SE_EV_VOLCANO, SFX_FLAG};
use oot_game::pack::keys;
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::quake::QUAKE_TYPE_1;

use crate::en_fire_rock::{ACTOR_EN_FIRE_ROCK, EnFireRock, FIRE_ROCK_SPAWNED_FALLING1, FIRE_ROCK_SPAWNED_FALLING2, OBJECT_EFC_STAR_FIELD};

/// `ACTOR_EN_ENCOUNT2` (`actor_table.h`: 0x00B4).
pub const ACTOR_EN_ENCOUNT2: i16 = 0x00B4;

/// `En_Encount2_Profile`: `ACTORCAT_ENEMY`, `ACTOR_FLAG_UPDATE_CULLING_DISABLED |
/// ACTOR_FLAG_DRAW_CULLING_DISABLED`, `OBJECT_EFC_STAR_FIELD`.
pub const PROFILE: ActorProfile = ActorProfile {
    id: ACTOR_EN_ENCOUNT2,
    name: "En_Encount2",
    category: ACTORCAT_ENEMY,
    flags: ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_DRAW_CULLING_DISABLED,
    object: crate::en_fire_rock::OBJECT,
    schema: &SCHEMA,
};

/// `params`: printed at init only.
pub const PARAMS: Field = Field::whole("params", Kind::Debug, "printed at init, used by nothing");

/// `EVENTCHKINF_49` (`save.h`: 0x49): the Fire Temple's blue warp used.
pub const EVENTCHKINF_49: u16 = 0x49;

/// En_Encount2's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[PARAMS],
    flags: &[
        FlagUse::new(Flag::Switch("0x37")).reads().note("Ganon's Tower's collapse: its rocks only once it's set"),
        FlagUse::new(Flag::EventChkInf("EVENTCHKINF_49", EVENTCHKINF_49)).reads().note("the Fire Temple's blue warp used: an adult's trail has no rocks (killed at init)"),
    ],
    todo: false,
    objects: &[],
    texts: &[],
};

/// `SCENE_*` (`scene_table.h`).
pub const SCENE_DEATH_MOUNTAIN_TRAIL: u16 = 0x60;
const SCENE_GANON_BOSS: u16 = 0x4F;
const SCENE_GANONS_TOWER_COLLAPSE_EXTERIOR: u16 = 0x1A;
const SCENE_GANONS_TOWER_COLLAPSE_INTERIOR: u16 = 0x0E;
const SCENE_INSIDE_GANONS_CASTLE_COLLAPSE: u16 = 0x0F;

/// `EN_ENCOUNT2_EFFECT_COUNT`.
pub const EN_ENCOUNT2_EFFECT_COUNT: usize = 50;

/// `Encount2State`.
const ENCOUNT2_INACTIVE: i16 = 0;
const ENCOUNT2_ACTIVE_DEATH_MOUNTAIN: i16 = 1;
const ENCOUNT2_ACTIVE_GANONS_TOWER: i16 = 2;

/// `EnEncount2Effect`: an ember.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct EnEncount2Effect {
    pub pos: Vec3,
    pub scale: f32,
    pub is_alive: bool,
    pub move_direction: Vec3,
    /// `rot`: degrees.
    pub rot: Vec3,
}

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `EnEncount2_Wait`.
    Wait,
    /// `EnEncount2_SpawnRocks`.
    SpawnRocks,
}

pub struct EnEncount2 {
    pub actor: Actor,
    pub action: Action,
    pub death_mountain_spawner_timer: i16,
    pub timer_between_rock_spawns: i16,
    pub num_spawned_rocks: i16,
    pub is_not_death_mountain: bool,
    /// `collapseSpawnerInactive`: set by `En_Zl3`.
    pub collapse_spawner_inactive: bool,
    pub effect_spawn_timer: i16,
    pub unk_160: f32,
    pub unk_168: f32,
    pub unk_170: f32,
    pub env_effects_timer: i16,
    pub unk_17c: f32,
    pub is_quaking: bool,
    pub effects: [EnEncount2Effect; EN_ENCOUNT2_EFFECT_COUNT],
}

/// The trail's area (`EnEncount2_Wait`'s, `EnEncount2_SpawnRocks`'s with z above `z_min`).
fn in_trail_area(p: Vec3, z_min: f32) -> bool {
    p.y > 1500.0 && p.x > -700.0 && p.x < 100.0 && p.z < -1290.0 && p.z > z_min
}

/// The collapse's scenes.
fn is_collapse_scene(scene_id: u16) -> bool {
    matches!(scene_id, SCENE_GANON_BOSS | SCENE_GANONS_TOWER_COLLAPSE_EXTERIOR | SCENE_GANONS_TOWER_COLLAPSE_INTERIOR | SCENE_INSIDE_GANONS_CASTLE_COLLAPSE)
}

/// What these functions read of Player: his position, `speedXZ` and `floorHeight`.
fn player_info(play: &PlayState) -> Option<(Vec3, f32, f32)> {
    let a = play.actors.get(play.player?)?;
    Some((a.base().world_pos, a.as_player().map(|p| p.speed_xz()).unwrap_or(0.0), a.base().floor_height))
}

/// `DEG_TO_RAD` (a double `M_PI`).
fn deg_to_rad(d: f32) -> f32 {
    (d as f64 * (std::f64::consts::PI / 180.0)) as f32
}

impl EnEncount2 {
    /// `EnEncount2_Init`.
    pub fn init(actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut this = EnEncount2 {
            actor,
            action: Action::Wait,
            death_mountain_spawner_timer: 0,
            timer_between_rock_spawns: 0,
            num_spawned_rocks: 0,
            is_not_death_mountain: false,
            collapse_spawner_inactive: false,
            effect_spawn_timer: 0,
            unk_160: 0.0,
            unk_168: 0.0,
            unk_170: 0.0,
            env_effects_timer: 0,
            unk_17c: 0.0,
            is_quaking: false,
            effects: [EnEncount2Effect::default(); EN_ENCOUNT2_EFFECT_COUNT],
        };
        if play.scene_id != SCENE_DEATH_MOUNTAIN_TRAIL {
            this.is_not_death_mountain = true;
        }
        let params = PARAMS.get(this.actor.params);
        if !this.is_not_death_mountain {
            log::debug!("En_Encount2: Death Mountain Encount2 set {params}");
            if play.save.adult && play.save.get_event_chk_inf(EVENTCHKINF_49) {
                this.actor.kill();
            }
        } else {
            log::debug!("En_Encount2: Ganon Tower Escape Encount2 set {params}");
        }
        this.action = Action::Wait;
        Box::new(this)
    }

    /// Ganon's Tower's check: Link within 700 across, switch 0x37, a collapse scene, not stopped.
    fn ganons_tower_active(&self, play: &PlayState) -> bool {
        self.actor.xz_dist_to_player < 700.0 && play.flags.get_switch(0x37) && is_collapse_scene(play.scene_id) && !self.collapse_spawner_inactive
    }

    /// `EnEncount2_Wait`: Link in the area: the quake (on the round's last frame, or none
    /// running), the darkening up, and once the wait is out a round of 200; out of it the
    /// darkening down.
    fn wait(&mut self, play: &mut PlayState) {
        let Some((player_pos, ..)) = player_info(play) else { return };
        let mut spawner_state = ENCOUNT2_INACTIVE;
        if !self.is_not_death_mountain {
            if in_trail_area(player_pos, -3600.0) {
                spawner_state = ENCOUNT2_ACTIVE_DEATH_MOUNTAIN;
            }
        } else if self.ganons_tower_active(play) {
            spawner_state = ENCOUNT2_ACTIVE_GANONS_TOWER;
        }
        match spawner_state {
            ENCOUNT2_INACTIVE => {
                self.is_quaking = false;
                self.env_effects_timer -= 1;
                if self.env_effects_timer <= 0 {
                    self.env_effects_timer = 0;
                }
            }
            _ => {
                if spawner_state == ENCOUNT2_ACTIVE_DEATH_MOUNTAIN && (self.death_mountain_spawner_timer == 1 || !self.is_quaking) {
                    let quake_index = play.quake_request(play.active_cam_id, QUAKE_TYPE_1);
                    play.quake_set_speed(quake_index, 0x7FFF);
                    play.quake_set_perturbations(quake_index, 50, 0, 0, 0);
                    play.quake_set_duration(quake_index, 300);
                    self.is_quaking = true;
                }
                // FALLTHROUGH (ENCOUNT2_ACTIVE_GANONS_TOWER).
                self.env_effects_timer += 1;
                if self.env_effects_timer > 60 {
                    self.env_effects_timer = 60;
                }
                if self.death_mountain_spawner_timer == 0 {
                    self.death_mountain_spawner_timer = 200;
                    self.num_spawned_rocks = 0;
                    self.action = Action::SpawnRocks;
                }
            }
        }
    }

    /// `EnEncount2_SpawnRocks`: the round (back to the wait, 100 frames, when it's out); with Link
    /// in the area an ember and, below the limit with the gap out, a rock.
    fn spawn_rocks(&mut self, play: &mut PlayState) {
        let Some((player_pos, player_speed, _)) = player_info(play) else { return };
        self.env_effects_timer += 1;
        if self.env_effects_timer > 60 {
            self.env_effects_timer = 60;
        }
        let mut spawner_state = ENCOUNT2_INACTIVE;
        let mut max_rocks = 0;
        if !self.is_not_death_mountain {
            if self.death_mountain_spawner_timer == 0 {
                self.death_mountain_spawner_timer = 100;
                self.action = Action::Wait;
                return;
            }
            if in_trail_area(player_pos, -3860.0) {
                max_rocks = 2;
                spawner_state = ENCOUNT2_ACTIVE_DEATH_MOUNTAIN;
            }
            audio_play_actor_sfx2(play, NA_SE_EV_VOLCANO - SFX_FLAG);
        } else if self.ganons_tower_active(play) {
            max_rocks = 1;
            spawner_state = ENCOUNT2_ACTIVE_GANONS_TOWER;
        }
        if spawner_state == ENCOUNT2_INACTIVE {
            return;
        }
        let (eye, at) = (play.view.eye, play.view.at);
        // The camera's facing, normalised.
        let d = at - eye;
        let magnitude = (d.x * d.x + d.y * d.y + d.z * d.z).sqrt();
        let dir = Vec3::new(d.x / magnitude, d.y / magnitude, d.z / magnitude);
        // 160 to 300 ahead of the camera by its pitch, 400 up.
        let ahead = Vec3::new(eye.x + (dir.x * 300.0), eye.y + (dir.y * 160.0) + 400.0, eye.z + (dir.z * 300.0));
        // 160 to 200 ahead, 400 up, at random.
        let ex = play.rand.centered_float(200.0) + (eye.x + (dir.x * 200.0));
        let ey = play.rand.centered_float(50.0) + ahead.y;
        let ez = play.rand.centered_float(200.0) + (eye.z + (dir.z * 200.0));
        let effect_scale = play.rand.centered_float(0.005) + 0.007;
        let effect_pos = Vec3::new(ex, ey, ez);
        if spawner_state == ENCOUNT2_ACTIVE_DEATH_MOUNTAIN {
            self.spawn_effect(play, effect_pos, effect_scale);
        } else if self.effect_spawn_timer == 0 {
            self.spawn_effect(play, effect_pos, effect_scale);
            self.effect_spawn_timer = 5;
        }
        if self.num_spawned_rocks < max_rocks && self.timer_between_rock_spawns == 0 {
            let (spawned_rock_type, x, z);
            if spawner_state == ENCOUNT2_ACTIVE_DEATH_MOUNTAIN {
                self.timer_between_rock_spawns = 4;
                let child_near = play.rand.zero_float(1.99) < 1.0 && !play.save.adult;
                if child_near {
                    x = play.rand.centered_float(10.0) + player_pos.x;
                    z = play.rand.centered_float(10.0) + player_pos.z;
                    spawned_rock_type = FIRE_ROCK_SPAWNED_FALLING1;
                } else {
                    if player_speed != 0.0 {
                        // 300 to 600 from the camera by its yaw.
                        x = play.rand.centered_float(200.0) + (eye.x + (dir.x * 300.0));
                        z = play.rand.centered_float(50.0) + (eye.z + (dir.z * 600.0));
                    } else {
                        x = play.rand.centered_float(10.0) + player_pos.x;
                        z = play.rand.centered_float(10.0) + player_pos.z;
                    }
                    spawned_rock_type = FIRE_ROCK_SPAWNED_FALLING2;
                }
            } else {
                self.timer_between_rock_spawns = 50;
                spawned_rock_type = FIRE_ROCK_SPAWNED_FALLING2;
                // About 300 ahead of the camera, or a quarter of the time near Link.
                let mut px = play.rand.centered_float(100.0) + ahead.x;
                let mut pz = play.rand.centered_float(100.0) + ahead.z;
                if play.rand.zero_float(3.99) < 1.0 {
                    px = play.rand.centered_float(70.0) + player_pos.x;
                    pz = play.rand.centered_float(70.0) + player_pos.z;
                }
                (x, z) = (px, pz);
            }
            let me = play.cur_actor;
            match play.actor_spawn_as_child(&mut self.actor, ACTOR_EN_FIRE_ROCK, Vec3::new(x, ahead.y, z), [0; 3], spawned_rock_type) {
                Ok(h) => {
                    if let Some(r) = play.actors.downcast_mut::<EnFireRock>(h) {
                        r.spawner = me;
                    }
                    self.num_spawned_rocks += 1;
                }
                Err(e) => log::debug!("En_Encount2: Can't spawn! ({e:?})"),
            }
        }
    }

    /// `EnEncount2_SpawnEffect`: an ember in the first free slot, 20 down a frame and ±10 across.
    fn spawn_effect(&mut self, play: &mut PlayState, position: Vec3, scale: f32) {
        if let Some(effect) = self.effects.iter_mut().find(|e| !e.is_alive) {
            effect.pos = position;
            effect.scale = scale;
            effect.rot = Vec3::ZERO;
            effect.move_direction.x = play.rand.centered_float(20.0);
            effect.move_direction.y = -20.0;
            effect.move_direction.z = play.rand.centered_float(20.0);
            effect.is_alive = true;
        }
    }

    /// `EnEncount2_UpdateEffects`: each ember tumbles (up to 500 degrees a frame each way) and
    /// moves (at most 30 across, 250 down); gone below y 1500 on the trail, 50 below Link's floor
    /// elsewhere.
    fn update_effects(&mut self, play: &mut PlayState) {
        let floor = player_info(play).map(|p| p.2).unwrap_or(0.0);
        let trail = play.scene_id == SCENE_DEATH_MOUNTAIN_TRAIL;
        for effect in self.effects.iter_mut().filter(|e| e.is_alive) {
            effect.rot.x += play.rand.zero_one() * 500.0;
            effect.rot.y += play.rand.zero_one() * 500.0;
            effect.rot.z += play.rand.zero_one() * 500.0;
            let target = effect.pos + effect.move_direction;
            approach_f(&mut effect.pos.x, target.x, 0.3, 30.0);
            approach_f(&mut effect.pos.y, target.y, 0.8, 250.0);
            approach_f(&mut effect.pos.z, target.z, 0.3, 30.0);
            approach_f(&mut effect.move_direction.y, -20.0, 0.9, 1.0);
            if !trail {
                if effect.pos.y < floor - 50.0 {
                    effect.is_alive = false;
                }
            } else if effect.pos.y < 1500.0 {
                effect.is_alive = false;
            }
        }
    }

    /// `EnEncount2_Update`'s darkening of the trail by `envEffectsTimer` (0 to 60).
    fn update_env(&mut self, play: &mut PlayState) {
        self.unk_17c = self.env_effects_timer as f32 / 60.0;
        self.unk_160 = self.unk_17c * -50.0;
        let e = &mut play.env_ctx;
        e.adj_ambient_color[0] = ((self.unk_160 as i16) as f32 * -1.5) as i16;
        e.adj_ambient_color[1] = self.unk_160 as i16;
        e.adj_ambient_color[2] = self.unk_160 as i16;
        self.unk_168 = self.unk_17c * -20.0;
        e.adj_light1_color[0] = ((self.unk_168 as i16) as f32 * -1.5) as i16;
        e.adj_light1_color[1] = self.unk_168 as i16;
        e.adj_light1_color[2] = self.unk_168 as i16;
        self.unk_170 = self.unk_17c * -50.0;
        e.adj_fog_near = self.unk_170 as i16;
        // @bug (game): (u8) of a negative product wraps.
        let fog = e.light_settings.fog_color;
        let targets = [160.0, 160.0, 150.0];
        for i in 0..3 {
            e.adj_fog_color[i] = (((targets[i] - fog[i] as f32) * self.unk_17c) as i32 as u8) as i16;
        }
    }
}

impl ActorImpl for EnEncount2 {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `EnEncount2_Update`.
    fn update(&mut self, play: &mut PlayState) {
        if self.death_mountain_spawner_timer != 0 {
            self.death_mountain_spawner_timer -= 1;
        }
        if self.timer_between_rock_spawns != 0 {
            self.timer_between_rock_spawns -= 1;
        }
        if self.effect_spawn_timer != 0 {
            self.effect_spawn_timer -= 1;
        }
        match self.action {
            Action::Wait => self.wait(play),
            Action::SpawnRocks => self.spawn_rocks(play),
        }
        self.update_effects(play);
        if !self.is_not_death_mountain {
            self.update_env(play);
        }
    }

    fn render_state(&self) -> RenderState {
        RenderState::of(&self.actor)
    }

    /// `EnEncount2_Draw` (`EnEncount2_DrawEffects`): with its object in, each ember at its place,
    /// tumbled (X, Y, Z), at its scale.
    fn draw(&self, _rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        // schema: its own profile object, object_efc_star_field
        if play.object_ctx.get_index(OBJECT_EFC_STAR_FIELD).is_none() {
            return;
        }
        for effect in self.effects.iter().filter(|e| e.is_alive) {
            let m = Mat4::from_translation(effect.pos)
                * Mat4::from_rotation_x(deg_to_rad(effect.rot.x))
                * Mat4::from_rotation_y(deg_to_rad(effect.rot.y))
                * Mat4::from_rotation_z(deg_to_rad(effect.rot.z))
                * Mat4::from_scale(Vec3::splat(effect.scale));
            out.opa.push(DrawCmd::new(MeshKey::named(keys::bake(crate::demo_kankyo::BAKE_ROCK)), m));
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
