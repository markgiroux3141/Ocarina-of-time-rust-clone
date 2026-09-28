//! The playable world: Player on static collision, the camera, and a fixed 20 Hz timestep
//! with snapshots for interpolated rendering.
//!
//! Two cameras run side by side: the game's (`camera::GameCamera`, `Camera_Normal1` from
//! `z_camera.c`) and spike 03's simple follow camera. Either can drive Player. Both keep the
//! game's contract with Player: Player reads the camera's input yaw from the previous frame
//! (`Camera_GetInputDirYaw`), because cameras update after actors.

use std::sync::Arc;

use glam::Vec3;

use crate::bg_ydan_hasi::BgYdanHasi;
use crate::bgcheck::StaticCollision;
use crate::camera::{GameCamera, PlayerView};
use crate::target::{TargetActor, TargetCtx, TargetFrame};
use crate::data::GameData;
use crate::input::{BTN_CLEFT, BTN_CRIGHT, Input, PadMgr, PadState};
use crate::math::{GAME_HZ, binang_to_rad, rad_to_binang};
use crate::player::{Env, LookRotations, Player};

/// Simple third-person follow camera, rotated with C-left / C-right.
#[derive(Debug, Clone, Copy)]
pub struct FollowCamera {
    /// Point the camera looks at.
    pub at: Vec3,
    /// Orbit yaw: direction from `at` to the eye, radians (0 = eye on +z side).
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    /// Height of the look-at point above Player's feet.
    pub height: f32,
}

impl FollowCamera {
    pub fn behind(pos: Vec3, facing: i16, adult: bool) -> FollowCamera {
        let height = if adult { 45.0 } else { 30.0 };
        // Eye behind Player: opposite the facing direction.
        let yaw = binang_to_rad(facing) + std::f32::consts::PI;
        FollowCamera { at: pos + Vec3::Y * height, yaw, pitch: 0.28, distance: if adult { 260.0 } else { 200.0 }, height }
    }

    pub fn eye(&self) -> Vec3 {
        let dir = Vec3::new(self.pitch.cos() * self.yaw.sin(), self.pitch.sin(), self.pitch.cos() * self.yaw.cos());
        self.at + dir * self.distance
    }

    /// `Camera_GetInputDirYaw`: `inputDir.y = atEyeGeo.yaw - 0x7FFF`, i.e. the yaw the
    /// camera looks along.
    pub fn input_dir_yaw(&self) -> i16 {
        rad_to_binang(self.yaw).wrapping_sub(0x7FFF)
    }

    /// One game frame: C-left/right orbit at 6° per frame, the look-at point follows Player
    /// with lag, and the orbit drifts behind Player while running.
    pub fn update(&mut self, input: &Input, player_pos: Vec3, player_facing: i16, speed: f32) {
        let step = 6f32.to_radians();
        if input.cur.held(BTN_CLEFT) {
            self.yaw += step;
        }
        if input.cur.held(BTN_CRIGHT) {
            self.yaw -= step;
        }
        let target = player_pos + Vec3::Y * self.height;
        self.at += (target - self.at) * 0.35;
        if speed > 1.0 && !input.cur.held(BTN_CLEFT) && !input.cur.held(BTN_CRIGHT) {
            let behind = binang_to_rad(player_facing) + std::f32::consts::PI;
            let mut d = (behind - self.yaw) % std::f32::consts::TAU;
            if d > std::f32::consts::PI {
                d -= std::f32::consts::TAU;
            } else if d < -std::f32::consts::PI {
                d += std::f32::consts::TAU;
            }
            // Only swing when Player runs roughly away from or across the view, not at it.
            if d.abs() < 2.4 {
                self.yaw += d * 0.02 * (speed / 6.0).min(1.0);
            }
        }
        self.yaw %= std::f32::consts::TAU;
    }
}

/// Which camera drives Player and the view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CameraKind {
    /// `z_camera.c` (`Camera_Normal1`).
    Game,
    /// Spike 03's follow camera (C-left / C-right orbit).
    Follow,
}

/// The view a camera produced this frame.
#[derive(Debug, Clone, Copy)]
pub struct CamView {
    pub eye: Vec3,
    pub at: Vec3,
    /// Vertical field of view, degrees.
    pub fov: f32,
}

/// What the renderer needs from one game frame.
#[derive(Debug, Clone)]
pub struct Snapshot {
    pub pos: Vec3,
    pub facing: i16,
    pub joints: oot_core::anim::JointTable,
    pub look: LookRotations,
    pub face: usize,
    pub speed_xz: f32,
    /// `shape.yOffset` (model units).
    pub y_offset: f32,
    /// `modelGroup` (PLAYER_MODELGROUP_*): which hand/sheath models are drawn.
    pub model_group: usize,
    pub camera: FollowCamera,
    pub view: CamView,
    /// Each bg actor's transform (what `Actor_Draw` sets up for the platforms).
    pub platforms: Vec<crate::dyna::ScaleRotPos>,
}

impl Snapshot {
    /// Interpolates towards `next` by `t` (0..1). Angles take the short way round.
    pub fn lerp(&self, next: &Snapshot, t: f32) -> Snapshot {
        let la = |a: i16, b: i16| a.wrapping_add((b.wrapping_sub(a) as f32 * t) as i16);
        let rot = self
            .joints
            .rot
            .iter()
            .zip(&next.joints.rot)
            .enumerate()
            .map(|(i, (a, b))| {
                if i == 0 {
                    // Root translation.
                    [0, 1, 2].map(|k| (a[k] as f32 + (b[k] as f32 - a[k] as f32) * t) as i16)
                } else {
                    [la(a[0], b[0]), la(a[1], b[1]), la(a[2], b[2])]
                }
            })
            .collect();
        let mut cam = self.camera;
        cam.at = self.camera.at.lerp(next.camera.at, t);
        let mut dy = next.camera.yaw - self.camera.yaw;
        if dy > std::f32::consts::PI {
            dy -= std::f32::consts::TAU;
        } else if dy < -std::f32::consts::PI {
            dy += std::f32::consts::TAU;
        }
        cam.yaw = self.camera.yaw + dy * t;
        Snapshot {
            pos: self.pos.lerp(next.pos, t),
            facing: la(self.facing, next.facing),
            joints: oot_core::anim::JointTable { rot, face: if t < 0.5 { self.joints.face } else { next.joints.face } },
            look: LookRotations {
                head: [la(self.look.head[0], next.look.head[0]), la(self.look.head[1], next.look.head[1]), la(self.look.head[2], next.look.head[2])],
                upper_y: la(self.look.upper_y, next.look.upper_y),
                upper_x: la(self.look.upper_x, next.look.upper_x),
                upper_z: la(self.look.upper_z, next.look.upper_z),
                root_pitch: la(self.look.root_pitch, next.look.root_pitch),
            },
            face: if t < 0.5 { self.face } else { next.face },
            speed_xz: self.speed_xz + (next.speed_xz - self.speed_xz) * t,
            y_offset: self.y_offset + (next.y_offset - self.y_offset) * t,
            model_group: if t < 0.5 { self.model_group } else { next.model_group },
            camera: cam,
            platforms: self
                .platforms
                .iter()
                .zip(&next.platforms)
                .map(|(a, b)| crate::dyna::ScaleRotPos { scale: a.scale.lerp(b.scale, t), rot: [0, 1, 2].map(|k| la(a.rot[k], b.rot[k])), pos: a.pos.lerp(b.pos, t) })
                .collect(),
            view: CamView {
                eye: self.view.eye.lerp(next.view.eye, t),
                at: self.view.at.lerp(next.view.at, t),
                fov: self.view.fov + (next.view.fov - self.view.fov) * t,
            },
        }
    }
}

pub struct World {
    pub data: Arc<GameData>,
    pub col: StaticCollision,
    pub player: Player,
    pub camera: FollowCamera,
    pub game_camera: GameCamera,
    pub camera_kind: CameraKind,
    /// Run `func_8008F87C` (foot IK) after each frame, as Player_Draw does.
    pub foot_ik: bool,
    /// The last frame's foot IK result per leg (left, right).
    pub legs: Option<[crate::footik::LegResult; 2]>,
    /// Targetable actors (dummy targets) and the target context (`z_actor.c`).
    pub targets: Vec<TargetActor>,
    pub target_ctx: TargetCtx,
    /// Moving platforms (`Bg_Ydan_Hasi` floating blocks), registered in `col.dyna`.
    pub platforms: Vec<BgYdanHasi>,
    pub pad: PadMgr,
    pub frames: u32,
    pub last_input: Input,
    spawn: (Vec3, i16),
    acc: f32,
    prev: Snapshot,
    cur: Snapshot,
}

impl World {
    pub fn new(data: Arc<GameData>, col: StaticCollision, adult: bool, spawn: Vec3, yaw: i16) -> World {
        let player = Player::new(&data, adult, spawn, yaw);
        let camera = FollowCamera::behind(spawn, yaw, adult);
        let game_camera = GameCamera::new(&data.camera, &Self::player_view(&data, &player));
        let snap = Self::snapshot_of(&player, &camera, &game_camera, CameraKind::Game, &col.dyna);
        World {
            data,
            col,
            player,
            camera,
            game_camera,
            camera_kind: CameraKind::Game,
            foot_ik: true,
            legs: None,
            targets: Vec::new(),
            target_ctx: TargetCtx::new(),
            platforms: Vec::new(),
            pad: PadMgr::default(),
            frames: 0,
            last_input: Input::default(),
            spawn: (spawn, yaw),
            acc: 0.0,
            prev: snap.clone(),
            cur: snap,
        }
    }

    fn player_view(data: &GameData, p: &Player) -> PlayerView {
        let age = if p.adult { 0 } else { 1 };
        PlayerView { pos: p.actor.world_pos, shape_yaw: p.actor.shape_rot.y, adult: p.adult, run_speed_limit: data.regs[age].reg(45) }
    }

    fn snapshot_of(p: &Player, cam: &FollowCamera, game: &GameCamera, kind: CameraKind, dyna: &crate::dyna::Dyna) -> Snapshot {
        let view = match kind {
            CameraKind::Game => CamView { eye: game.eye, at: game.at, fov: game.fov },
            CameraKind::Follow => CamView { eye: cam.eye(), at: cam.at, fov: 50.0 },
        };
        Snapshot {
            pos: p.actor.world_pos,
            facing: p.actor.shape_rot.y,
            joints: p.draw_joints(),
            look: p.look_rotations(),
            face: p.face,
            speed_xz: p.actor.speed_xz,
            y_offset: p.actor.shape_y_offset,
            model_group: p.model_group,
            camera: *cam,
            view,
            platforms: dyna.actors.iter().map(|a| a.cur).collect(),
        }
    }

    /// Spawns a `Bg_Ydan_Hasi` floating block with `header` (`gDTSlidingPlatformCol`) at
    /// `home`, floating on `water_surface`, and builds its collision for the next frame.
    pub fn spawn_platform(&mut self, header: Arc<oot_core::collision::CollisionHeader>, home: Vec3, yaw: i16, water_surface: f32) {
        let p = BgYdanHasi::spawn(&mut self.col.dyna, header, home, yaw, water_surface);
        self.platforms.push(p);
        self.col.dyna.update_context();
        let s = Self::snapshot_of(&self.player, &self.camera, &self.game_camera, self.camera_kind, &self.col.dyna);
        self.prev = s.clone();
        self.cur = s;
    }

    /// Switches between the game camera and the follow camera.
    pub fn toggle_camera(&mut self) {
        self.camera_kind = match self.camera_kind {
            CameraKind::Game => CameraKind::Follow,
            CameraKind::Follow => CameraKind::Game,
        };
        let s = Self::snapshot_of(&self.player, &self.camera, &self.game_camera, self.camera_kind, &self.col.dyna);
        self.prev = s.clone();
        self.cur = s;
    }

    /// `Camera_GetInputDirYaw` of the active camera.
    pub fn input_dir_yaw(&self) -> i16 {
        match self.camera_kind {
            CameraKind::Game => self.game_camera.input_dir_yaw(),
            CameraKind::Follow => self.camera.input_dir_yaw(),
        }
    }

    pub fn respawn(&mut self) {
        let (pos, yaw) = self.spawn;
        let adult = self.player.adult;
        self.player = Player::new(&self.data, adult, pos, yaw);
        self.camera = FollowCamera::behind(pos, yaw, adult);
        self.game_camera = GameCamera::new(&self.data.camera, &Self::player_view(&self.data, &self.player));
        let s = Self::snapshot_of(&self.player, &self.camera, &self.game_camera, self.camera_kind, &self.col.dyna);
        self.prev = s.clone();
        self.cur = s;
    }

    /// Feeds one controller poll (call at the device rate, e.g. every display frame).
    pub fn poll(&mut self, pad: PadState) {
        self.pad.poll(pad);
    }

    /// Runs one game frame with the input accumulated since the last one.
    pub fn tick(&mut self) {
        let input = self.pad.request();
        self.tick_with(input);
    }

    /// One game frame with an explicit input (scripted tests).
    pub fn tick_with(&mut self, input: Input) {
        self.frames += 1;
        self.last_input = input;
        // Actor_UpdateAll: the BG category (the platforms) updates before Player, then
        // DynaPoly_UpdateContext rebuilds their collision.
        if !self.platforms.is_empty() {
            for p in &mut self.platforms {
                p.update(self.frames);
                self.col.dyna.set_source(p.bg, p.source());
            }
            self.col.dyna.update_context();
        }
        let env = Env {
            data: &self.data,
            col: &self.col,
            cam_input_yaw: self.input_dir_yaw(),
            gameplay_frames: self.frames,
            targets: &self.targets,
            target: self.target_ctx.view(),
        };
        self.player.update(&env, input);
        self.player.finish_frame();
        if self.foot_ik {
            self.legs = Some(self.player.apply_foot_ik(&self.data, &self.col));
        }
        // Actor_UpdateAll: the other actors' distances to Player, then the target context.
        for t in &mut self.targets {
            t.update_distances(self.player.actor.world_pos);
        }
        // Play_Update: cameras update after the actors.
        self.camera.update(&input, self.player.actor.world_pos, self.player.actor.shape_rot.y, self.player.actor.speed_xz);
        let pv = Self::player_view(&self.data, &self.player);
        self.game_camera.update(&self.data.camera, &self.col, &pv, self.frames);
        let (eye, at, fov) = match self.camera_kind {
            CameraKind::Game => (self.game_camera.eye, self.game_camera.at, self.game_camera.fov),
            CameraKind::Follow => (self.camera.eye(), self.camera.at, 50.0),
        };
        // play->viewProjectionMtxF: the game's projection (320x240, zNear 10).
        let view_proj = oot_core::room::gu_perspective(fov, 4.0 / 3.0, 10.0, 12800.0) * glam::camera::rh::view::look_at_mat4(eye, at, Vec3::Y);
        let frame = TargetFrame {
            col: &self.col,
            ranges: &self.data.target_ranges,
            player_target: self.player.unk_664,
            player_timer: self.player.unk_66C,
            player_stick_dir: self.player.unk_84B[self.player.unk_846 as usize],
            player_shape_yaw: self.player.actor.shape_rot.y,
            player_focus: self.player.head_pos,
            view_proj,
        };
        crate::target::update(&mut self.target_ctx, &self.targets, &frame);
        // DynaPoly_UpdateBgActorTransforms, at the end of Actor_UpdateAll.
        self.col.dyna.update_prev_transforms();
        // Fell out of the course: respawn.
        if self.player.actor.world_pos.y < -2000.0 {
            self.respawn();
            return;
        }
        self.prev = std::mem::replace(&mut self.cur, Self::snapshot_of(&self.player, &self.camera, &self.game_camera, self.camera_kind, &self.col.dyna));
    }

    /// Advances real time by `dt` seconds, running as many 20 Hz frames as are due (at most
    /// 5 per call). Returns the number of frames run.
    pub fn advance(&mut self, dt: f32) -> u32 {
        self.acc += dt.min(0.25);
        let step = 1.0 / GAME_HZ;
        let mut n = 0;
        while self.acc >= step && n < 5 {
            self.acc -= step;
            self.tick();
            n += 1;
        }
        n
    }

    /// Render state between the last two game frames.
    pub fn render_snapshot(&self) -> Snapshot {
        let t = (self.acc * GAME_HZ).clamp(0.0, 1.0);
        self.prev.lerp(&self.cur, t)
    }

    pub fn current_snapshot(&self) -> Snapshot {
        self.cur.clone()
    }
}

/// Builds the game frame's `Input` from a scripted pad state, holding `prev` for edge
/// detection (what `PadMgr` would produce if polled once per frame).
pub fn scripted_input(prev: PadState, cur: PadState) -> Input {
    let mut p = PadMgr::default();
    p.poll(prev);
    p.request();
    p.poll(cur);
    p.request()
}
