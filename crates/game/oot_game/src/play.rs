//! `PlayState` (`z_play.c`): everything in play, and the frame.
//!
//! ## The frame
//!
//! `PlayState::tick_with` runs one game frame (20 Hz, `R_UPDATE_RATE` 3) in the decomp's order
//! (`Play_Update`, then the state `Play_Draw` leaves behind):
//!
//! 1. The transition (`crate::play_scene`: a scene's fade in or out), `Object_UpdateBank`,
//!    `gameplayFrames++`, the frame's input, and a room load finishing (`func_800973FC`).
//! 2. `Actor_UpdateAll`:
//!    - first the room's actor list, if a room just loaded (`numSetupActors`);
//!    - for each category in order (switch, BG, player, explosive, NPC, enemy, prop, item
//!      action, misc, boss, door, chest), each actor newest first: `prevPos`, the distances
//!      and yaw to Player, then its update if it's due (`freezeTimer` 0, `ACTOR_FLAG_4` or
//!      `ACTOR_FLAG_6`). Killed actors are deleted, actors waiting for their object initialise
//!      once it's loaded (and skip this frame), and actors whose object went are killed.
//!      Player updates in its category, before the later ones, so they see where it went;
//!    - after the BG category, `DynaPoly_UpdateContext`;
//!    - the target context (`func_8002C7BC`), with `viewProjectionMtxF` from the last drawn
//!      frame;
//!    - `DynaPoly_UpdateBgActorTransforms`.
//! 3. `AnimationContext_Update`: every actor's queued animation requests (Player's joint copies,
//!    blends and root motion).
//! 4. The cameras: the spikes' follow camera, and `Camera_Update` (`Camera_Normal1`).
//! 5. What `Play_Draw` changes: each actor's draw-time state (Player's foot IK writes into its
//!    joint table, so it's done once per game frame, not per rendered frame), the scene draw
//!    config (`Scene_Draw`: this frame's texture scrolls and colours), and the view the next
//!    frame's target context reads.
//! 6. Without a scene from the pack (the test course), the sandbox's void-out: below y −2000,
//!    Player respawns. In a scene Player's own exit and void checks do this.
//! 7. The render state of every actor and the camera is captured for blending.
//! 8. If the transition ended the scene, `Play_Init` for the next entrance replaces this play
//!    state (`PlayState::reinit`).
//!
//! The spikes' `World` ran step 3 and the foot IK right after Player's own update, and the
//! target context after the cameras with the new view. Moving them to where the decomp has
//! them changed none of the golden traces or renders, nor any test.
//!
//! ## Rendering
//!
//! - **Blending:** the renderer runs at the display rate and blends between the last two
//!   captured frames (`render_frame`). Each actor's `RenderState` is generic: position,
//!   rotation, scale, an optional joint table, and extra angles, values and switches the
//!   actor's draw function interprets. An actor that teleported (spawned, respawned) isn't
//!   blended.
//! - **Drawing:** `draw` asks every actor for its draws from its blended state, into OPA and
//!   XLU lists (`eng_gfx::DrawLists`), in `Actor_DrawAll` order, then the target reticle.

use std::sync::Arc;

use eng_collision::bgcheck::CollisionContext;
use eng_gfx::DrawLists;
use eng_input::pad::{Input, PadMgr, PadState};
use eng_math::GAME_HZ;
use glam::{Mat4, Vec3};

use crate::actor::{ACTOR_FLAG_4, ACTOR_FLAG_6, Actor};
use crate::actor_ctx::{ACTORCAT_BG, ACTORCAT_MAX, ActorContext, ActorHandle, ActorImpl};
use crate::camera::{CamView, CameraKind, FollowCamera, GameCamera, PlayerView};
use crate::data::GameData;
use crate::object_ctx::ObjectContext;
use crate::play_scene::{GameAssets, SceneState, TransitionState};
use crate::player_lib::PlayerRules;
use crate::room::RoomContext;
use crate::save::SaveContext;
use crate::scene::{ActorEntry, TransitionActorEntry};
use crate::spawn::{SceneFlags, Uninit};
use crate::target::{TargetCtx, TargetFrame};

/// The draw lists actors submit into.
pub type DrawOut = DrawLists;

/// Meshes the app builds itself rather than loading from the pack.
pub mod builtin {
    /// A soft dark disc of radius 1 (the circle shadow's stand-in).
    pub const SHADOW: &str = "builtin/shadow";
    /// The sandbox's dummy target: a box 30 wide and 60 high.
    pub const TARGET_BOX: &str = "builtin/target_box";
    /// An actor that isn't ported yet (`spawn::Placeholder`, with `Debug::placeholders`).
    pub const PLACEHOLDER: &str = "builtin/placeholder";
}

/// Where the frame is seen from, for draw code that faces the camera.
#[derive(Debug, Clone, Copy)]
pub struct ViewInfo {
    pub eye: Vec3,
}

/// What the renderer blends between two game frames for one actor.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RenderState {
    pub pos: Vec3,
    /// Shape rotation.
    pub rot: [i16; 3],
    pub scale: Vec3,
    /// A skeleton's joint table (entry 0 the root translation).
    pub joints: Option<eng_anim::anim::JointTable>,
    /// Blended the short way round (binary angles).
    pub angles: Vec<i16>,
    /// Blended linearly.
    pub values: Vec<f32>,
    /// Switched halfway.
    pub switches: Vec<u32>,
    /// Don't blend into this state.
    pub teleported: bool,
}

/// `a` to `b` by `t`, the short way round.
fn lerp_angle(a: i16, b: i16, t: f32) -> i16 {
    a.wrapping_add((b.wrapping_sub(a) as f32 * t) as i16)
}

impl RenderState {
    /// Position, shape rotation and scale of an actor.
    pub fn of(a: &Actor) -> RenderState {
        RenderState {
            pos: a.world_pos,
            rot: [a.shape_rot.x, a.shape_rot.y, a.shape_rot.z],
            scale: a.scale,
            teleported: a.teleported,
            ..Default::default()
        }
    }

    /// Blends towards `next` by `t` in 0..1.
    pub fn lerp(&self, next: &RenderState, t: f32) -> RenderState {
        if next.teleported {
            return next.clone();
        }
        RenderState {
            pos: self.pos.lerp(next.pos, t),
            rot: [0, 1, 2].map(|k| lerp_angle(self.rot[k], next.rot[k], t)),
            scale: self.scale.lerp(next.scale, t),
            joints: match (&self.joints, &next.joints) {
                (Some(a), Some(b)) => Some(a.lerp(b, t)),
                (_, b) => b.clone(),
            },
            angles: self.angles.iter().zip(&next.angles).map(|(&a, &b)| lerp_angle(a, b, t)).collect(),
            values: self.values.iter().zip(&next.values).map(|(&a, &b)| a + (b - a) * t).collect(),
            switches: if t < 0.5 { self.switches.clone() } else { next.switches.clone() },
            teleported: false,
        }
    }
}

/// The blended state of a frame: every actor's, and the camera's.
#[derive(Debug, Clone)]
pub struct RenderFrame {
    /// In `Actor_DrawAll` order.
    pub actors: Vec<(ActorHandle, RenderState)>,
    pub view: CamView,
    /// The spike's follow camera, for its own render mode.
    pub follow: FollowCamera,
}

impl RenderFrame {
    pub fn lerp(&self, next: &RenderFrame, t: f32) -> RenderFrame {
        let mut follow = self.follow;
        follow.at = self.follow.at.lerp(next.follow.at, t);
        let mut dy = next.follow.yaw - self.follow.yaw;
        if dy > std::f32::consts::PI {
            dy -= std::f32::consts::TAU;
        } else if dy < -std::f32::consts::PI {
            dy += std::f32::consts::TAU;
        }
        follow.yaw = self.follow.yaw + dy * t;
        RenderFrame {
            actors: next
                .actors
                .iter()
                .map(|(h, b)| match self.actors.iter().find(|(p, _)| p == h) {
                    Some((_, a)) => (*h, a.lerp(b, t)),
                    None => (*h, b.clone()),
                })
                .collect(),
            view: CamView {
                eye: self.view.eye.lerp(next.view.eye, t),
                at: self.view.at.lerp(next.view.at, t),
                fov: self.view.fov + (next.view.fov - self.view.fov) * t,
            },
            follow,
        }
    }

    pub fn actor(&self, h: ActorHandle) -> Option<&RenderState> {
        self.actors.iter().find(|(x, _)| *x == h).map(|(_, r)| r)
    }
}

/// Debug switches.
#[derive(Debug, Clone, Copy)]
pub struct Debug {
    /// Run `func_8008F87C` (foot IK) as `Player_Draw` does.
    pub foot_ik: bool,
    /// Draw a marker where each placeholder (unported actor) is.
    pub placeholders: bool,
}

/// Everything in play: `PlayState`.
pub struct PlayState {
    pub data: Arc<GameData>,
    pub rules: Arc<PlayerRules>,
    /// `colCtx`: the scene's static collision and the DynaPoly meshes.
    pub col: CollisionContext,
    /// `actorCtx`.
    pub actors: ActorContext,
    /// `actorCtx.actorLists[ACTORCAT_PLAYER].head`.
    pub player: Option<ActorHandle>,
    /// `mainCamera` (`Camera_Normal1`).
    pub game_camera: GameCamera,
    /// The spikes' follow camera, and which camera drives Player and the view.
    pub follow_camera: FollowCamera,
    pub camera_kind: CameraKind,
    /// `actorCtx.targetCtx`.
    pub target_ctx: TargetCtx,
    /// `gameplayFrames`.
    pub gameplay_frames: u32,
    /// `state.input[0]` for this frame, and the pad manager that builds it.
    pub input: Input,
    pub pad: PadMgr,
    pub debug: Debug,
    /// Where Player respawns (the spawn it entered at).
    pub spawn: (Vec3, i16),
    /// Rebuilds Player at `spawn` (set by the content crate, which knows Player).
    pub respawn_player: Option<fn(&mut PlayState)>,
    /// `viewProjectionMtxF`: the view the last frame was drawn with.
    pub view_proj: Mat4,
    /// `gSaveContext`.
    pub save: SaveContext,
    /// The pack's tables and the ported actors, when play entered a scene from the pack
    /// (`play_init`). Without them there's no spawning by id and no scene changes.
    pub assets: Option<Arc<GameAssets>>,
    /// The loaded scene (`play->loadedScene`, its header, its rooms), if any.
    pub scene: Option<SceneState>,
    /// `sceneId`, `curSpawn`.
    pub scene_id: u16,
    pub cur_spawn: usize,
    /// `roomCtx`, `objectCtx`.
    pub room_ctx: RoomContext,
    pub object_ctx: ObjectContext,
    /// Link's object for his age (`Scene_CommandSpawnList`).
    pub link_object_id: i16,
    /// `transiActorCtx.list`: the scene's transition actors (an entry's id is negated while
    /// its actor exists).
    pub transi_actors: Vec<TransitionActorEntry>,
    /// `setupActorList`: a loaded room's actors, spawned by the next `Actor_UpdateAll`.
    pub setup_actors: Vec<ActorEntry>,
    /// `actorCtx.flags`.
    pub flags: SceneFlags,
    /// `transitionTrigger`, `transitionMode`, `transitionType`, `nextEntranceIndex` and the
    /// running transition.
    pub transition: TransitionState,
    /// `unk_11E18`: the vertical room-change planes' screen dimming.
    pub unk_11e18: i16,
    /// How many scene changes led here (the renderer reloads its meshes when it changes).
    pub scene_changes: u32,
    pub(crate) next_play_init: bool,
    acc: f32,
    prev: Option<RenderFrame>,
    cur: Option<RenderFrame>,
}

impl PlayState {
    /// An empty play state over `col`. Spawn Player (and the rest) with the content crate,
    /// then call `reset_blending`.
    pub fn new(data: Arc<GameData>, rules: Arc<PlayerRules>, col: CollisionContext, spawn: (Vec3, i16), adult: bool) -> PlayState {
        let pv = PlayerView { pos: spawn.0, shape_yaw: spawn.1, adult, run_speed_limit: data.regs[if adult { 0 } else { 1 }].reg(45) };
        let game_camera = GameCamera::new(&data.camera, &pv);
        PlayState {
            follow_camera: FollowCamera::behind(spawn.0, spawn.1, adult),
            game_camera,
            camera_kind: CameraKind::Game,
            data,
            rules,
            col,
            actors: ActorContext::default(),
            player: None,
            target_ctx: TargetCtx::new(),
            gameplay_frames: 0,
            input: Input::default(),
            pad: PadMgr::default(),
            debug: Debug { foot_ik: true, placeholders: false },
            spawn,
            respawn_player: None,
            view_proj: Mat4::IDENTITY,
            save: SaveContext::new(0, adult, crate::env::clock_time(10, 0) as u16),
            assets: None,
            scene: None,
            scene_id: 0,
            cur_spawn: 0,
            room_ctx: RoomContext::default(),
            object_ctx: ObjectContext::default(),
            link_object_id: 0,
            transi_actors: Vec::new(),
            setup_actors: Vec::new(),
            flags: SceneFlags::default(),
            transition: TransitionState::default(),
            unk_11e18: 0,
            scene_changes: 0,
            next_play_init: false,
            acc: 0.0,
            prev: None,
            cur: None,
        }
    }

    /// Adds an actor (constructed and initialised) to the actor context.
    pub fn spawn(&mut self, actor: Box<dyn ActorImpl>) -> Option<ActorHandle> {
        let is_player = actor.as_player().is_some();
        let h = self.actors.insert(actor)?;
        if is_player {
            self.player = Some(h);
        }
        Some(h)
    }

    /// The player's view for the camera (`Camera_InitPlayerSettings`, `Camera_Update`).
    pub fn player_view(&self) -> Option<PlayerView> {
        let h = self.player?;
        let p = self.actors.get(h)?;
        let adult = p.as_player()?.adult();
        let a = p.base();
        Some(PlayerView { pos: a.world_pos, shape_yaw: a.shape_rot.y, adult, run_speed_limit: self.data.regs[if adult { 0 } else { 1 }].reg(45) })
    }

    /// Puts both cameras behind Player (`Camera_Init` and the follow camera's start).
    pub fn reset_cameras(&mut self) {
        if let Some(pv) = self.player_view() {
            self.follow_camera = FollowCamera::behind(pv.pos, pv.shape_yaw, pv.adult);
            self.game_camera = GameCamera::new(&self.data.camera, &pv);
        }
        self.view_proj = self.camera_view_proj();
    }

    /// `Camera_GetInputDirYaw` of the active camera.
    pub fn input_dir_yaw(&self) -> i16 {
        match self.camera_kind {
            CameraKind::Game => self.game_camera.input_dir_yaw(),
            CameraKind::Follow => self.follow_camera.input_dir_yaw(),
        }
    }

    /// Switches between the game camera and the follow camera.
    pub fn toggle_camera(&mut self) {
        self.camera_kind = match self.camera_kind {
            CameraKind::Game => CameraKind::Follow,
            CameraKind::Follow => CameraKind::Game,
        };
        self.view_proj = self.camera_view_proj();
        self.reset_blending();
    }

    /// Rebuilds Player at its spawn and puts the cameras behind it.
    pub fn respawn(&mut self) {
        if let Some(f) = self.respawn_player {
            f(self);
        }
        self.reset_cameras();
        self.reset_blending();
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

    /// One actor's turn in `Actor_UpdateAll`.
    fn update_actor(&mut self, h: ActorHandle, player_pos: Option<Vec3>) {
        let Some(mut a) = self.actors.take(h) else { return };
        let base = a.base_mut();
        if base.world_pos.y < -25000.0 {
            base.world_pos.y = -25000.0;
        }
        if let Some(u) = a.as_any_mut().downcast_mut::<Uninit>() {
            // actor->init != NULL: initialise once the object is loaded, and skip this frame.
            if u.actor.obj_bank_index.is_some_and(|b| self.object_ctx.is_loaded(b)) {
                let Ok(u) = a.into_any().downcast::<Uninit>() else { unreachable!() };
                let ctor = u.ctor;
                let init = ctor(u.actor, self);
                self.actors.put_back(h, init);
            } else {
                self.actors.put_back(h, a);
            }
            return;
        }
        if base_bank_dropped(a.base(), &self.object_ctx) {
            a.base_mut().kill();
        }
        let base = a.base_mut();
        if base.killed {
            // update == NULL: Actor_Delete (isDrawn isn't tracked, so it goes at once).
            a.destroy(self);
            self.actors.put_back(h, a);
            self.actors.remove(h);
            if self.player == Some(h) {
                self.player = None;
            }
            return;
        }
        base.prev_pos = base.world_pos;
        if let Some(pp) = player_pos {
            base.update_distances(pp);
        }
        base.flags &= !crate::actor::ACTOR_FLAG_24;
        // Culling (func_800314D4, which sets ACTOR_FLAG_6 for actors in view) isn't ported:
        // every actor counts as in view.
        base.flags |= ACTOR_FLAG_6;
        let due = if base.freeze_timer > 0 {
            base.freeze_timer -= 1;
            base.freeze_timer == 0
        } else {
            true
        };
        if due && base.flags & (ACTOR_FLAG_4 | ACTOR_FLAG_6) != 0 {
            if let Some(t) = self.player_target() {
                base.is_targeted = t == h;
            }
            a.update(self);
        }
        self.actors.put_back(h, a);
    }

    /// Player's `unk_664`.
    pub fn player_target(&self) -> Option<ActorHandle> {
        self.actors.get(self.player?)?.as_player()?.target()
    }

    /// One game frame with an explicit input (scripted runs and tests).
    pub fn tick_with(&mut self, input: Input) {
        self.update_transition();
        self.object_ctx.update_bank();
        self.gameplay_frames += 1;
        self.input = input;
        self.room_finish_load();
        self.update_all_actors();
        // AnimationContext_Update: every actor's queued animation requests.
        for h in self.actors.all() {
            if let Some(a) = self.actors.get_mut(h) {
                a.animation_update();
            }
        }
        // Play_Update: the cameras (they follow Player).
        if let Some(p) = self.player.and_then(|ph| self.actors.get(ph)) {
            let (pos, facing, speed) = (p.base().world_pos, p.base().shape_rot.y, p.as_player().map(|i| i.speed_xz()).unwrap_or(0.0));
            self.follow_camera.update(&input, pos, facing, speed);
            if let Some(pv) = self.player_view() {
                self.game_camera.update(&self.data.camera, &self.col, &pv, self.gameplay_frames);
            }
        }
        // Play_Draw: the actors' draw-time state (Player's foot IK), and the view it sets up
        // (play->viewProjectionMtxF), which the next frame's target context reads.
        for h in self.actors.all() {
            if let Some(mut a) = self.actors.take(h) {
                a.draw_update(self);
                self.actors.put_back(h, a);
            }
        }
        let frames = self.gameplay_frames;
        if let Some(s) = &mut self.scene {
            s.run_draw_config(frames);
        }
        self.view_proj = self.camera_view_proj();
        // The sandbox's void-out (a scene from the pack has Player's own).
        if self.assets.is_none() && self.player.and_then(|ph| self.actors.actor(ph)).is_some_and(|a| a.world_pos.y < -2000.0) {
            self.respawn();
            return;
        }
        self.prev = self.cur.take();
        self.cur = Some(self.capture());
        if self.next_play_init {
            self.reinit();
        }
    }

    /// `Actor_UpdateAll`.
    fn update_all_actors(&mut self) {
        self.spawn_setup_actors();
        for cat in 0..ACTORCAT_MAX {
            // A snapshot of the list: actors spawned now go to the head and wait for the
            // next frame, as with the decomp's linked lists.
            let list = self.actors.category(cat).to_vec();
            for h in list {
                // Player updates before the later categories, so they see its new position.
                let player_pos = self.player.and_then(|p| self.actors.actor(p)).map(|a| a.world_pos);
                self.update_actor(h, player_pos);
            }
            if cat == ACTORCAT_BG {
                self.col.dyna.update_context();
            }
        }
        // func_8002C7BC with the actor Player keeps targeted, against the last frame's view.
        if let Some(ph) = self.player
            && let Some(pi) = self.actors.get(ph).and_then(|p| p.as_player())
        {
            let frame = TargetFrame {
                col: &self.col,
                ranges: &self.data.target_ranges,
                player: ph,
                player_target: pi.target(),
                player_timer: pi.target_timer(),
                player_stick_dir: pi.stick_dir(),
                player_shape_yaw: self.actors.actor(ph).map(|a| a.shape_rot.y).unwrap_or(0),
                player_focus: pi.focus(),
                view_proj: self.view_proj,
            };
            crate::target::update(&mut self.target_ctx, &self.actors, &frame);
        }
        self.col.dyna.update_prev_transforms();
    }

    /// `play->viewProjectionMtxF` for the active camera: the game's 320x240 view, `zNear` 10.
    fn camera_view_proj(&self) -> Mat4 {
        let (eye, at, fov) = match self.camera_kind {
            CameraKind::Game => (self.game_camera.eye, self.game_camera.at, self.game_camera.fov),
            CameraKind::Follow => (self.follow_camera.eye(), self.follow_camera.at, 50.0),
        };
        eng_math::gu_perspective(fov, 4.0 / 3.0, 10.0, 12800.0) * glam::camera::rh::view::look_at_mat4(eye, at, Vec3::Y)
    }

    /// The render state of every actor and the camera now.
    fn capture(&mut self) -> RenderFrame {
        let mut actors = Vec::new();
        for h in self.actors.all() {
            if let Some(a) = self.actors.get_mut(h) {
                actors.push((h, a.render_state()));
                a.base_mut().teleported = false;
            }
        }
        let view = match self.camera_kind {
            CameraKind::Game => CamView { eye: self.game_camera.eye, at: self.game_camera.at, fov: self.game_camera.fov },
            CameraKind::Follow => CamView { eye: self.follow_camera.eye(), at: self.follow_camera.at, fov: 50.0 },
        };
        RenderFrame { actors, view, follow: self.follow_camera }
    }

    /// Starts blending afresh from now (after spawning, respawning or switching cameras).
    pub fn reset_blending(&mut self) {
        let f = self.capture();
        self.prev = Some(f.clone());
        self.cur = Some(f);
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

    /// The render state between the last two game frames.
    pub fn render_frame(&self) -> RenderFrame {
        let t = (self.acc * GAME_HZ).clamp(0.0, 1.0);
        match (&self.prev, &self.cur) {
            (Some(p), Some(c)) => p.lerp(c, t),
            (_, Some(c)) => c.clone(),
            _ => RenderFrame { actors: Vec::new(), view: CamView { eye: Vec3::ZERO, at: Vec3::Z, fov: 60.0 }, follow: self.follow_camera },
        }
    }

    /// The last game frame's render state, unblended.
    pub fn current_frame(&self) -> RenderFrame {
        self.cur.clone().unwrap_or_else(|| self.render_frame())
    }

    /// `Actor_DrawAll` (every actor from its render state `frame`), then the target reticle.
    pub fn draw(&self, frame: &RenderFrame, view: &ViewInfo, out: &mut DrawOut) {
        for (h, rs) in &frame.actors {
            if let Some(a) = self.actors.get(*h)
                && !a.base().killed
            {
                a.draw(rs, self, view, out);
            }
        }
        crate::target::draw_reticle(&self.target_ctx, &self.actors, self.gameplay_frames, view.eye, out);
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

/// `Actor_Draw`'s model matrix: translate, rotate by the shape yaw, scale.
pub fn actor_matrix(pos: Vec3, yaw: i16, scale: f32) -> Mat4 {
    Mat4::from_translation(pos) * Mat4::from_rotation_y(eng_math::binang_to_rad(yaw)) * Mat4::from_scale(Vec3::splat(scale))
}

/// `!Object_IsLoaded(&play->objectCtx, actor->objBankIndex)` for an initialised actor that
/// spawned by id (actors built directly have no bank).
fn base_bank_dropped(a: &Actor, objects: &ObjectContext) -> bool {
    a.obj_bank_index.is_some_and(|b| !objects.is_loaded(b))
}
