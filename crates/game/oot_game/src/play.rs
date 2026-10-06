//! `PlayState` (`z_play.c`): everything in play, and the frame.
//!
//! ## The frame
//!
//! `PlayState::tick_with` runs one game frame (20 Hz, `R_UPDATE_RATE` 3) in the decomp's order
//! (`Play_Update`, then the state `Play_Draw` leaves behind):
//!
//! 1. The transition (`crate::play_scene`: a scene's fade in or out), `Object_UpdateEntries`,
//!    `gameplayFrames++`, the frame's input, a room load finishing (`Room_ProcessRoomRequest`), and the
//!    collision check over what the actors registered last frame (`CollisionCheck_AT`, `_OC`,
//!    `_Damage`, then `_ClearContext`; `crate::collision_check`).
//! 2. `Actor_UpdateAll`:
//!    - first the room's actor list, if a room just loaded (`numSetupActors`);
//!    - for each category in order (switch, BG, player, explosive, NPC, enemy, prop, item
//!      action, misc, boss, door, chest), each actor newest first: `prevPos`, the distances
//!      and yaw to Player, then its update if it's due (`freezeTimer` 0, `ACTOR_FLAG_UPDATE_CULLING_DISABLED` or
//!      `ACTOR_FLAG_INSIDE_CULLING_VOLUME`), then `CollisionCheck_ResetDamage`. Killed actors are deleted, actors
//!      waiting for their object initialise once it's loaded (and skip this frame), and actors
//!      whose object went are killed.
//!      Player updates in its category, before the later ones, so they see where it went;
//!    - after the BG category, `DynaPoly_UpdateContext`;
//!    - the target context (`Attention_Update`), with `viewProjectionMtxF` from the last drawn
//!      frame;
//!    - `DynaPoly_UpdateBgActorTransforms`.
//! 3. `Message_Update` (`crate::message`), then `Interface_Update` (`crate::interface`: the
//!    buttons' status, the alpha fades, the health and rupee counters, the A button's flip).
//! 4. `AnimTaskQueue_Update`: every actor's queued animation requests (Player's joint copies,
//!    blends and root motion).
//! 5. `Letterbox_Update`, then the cameras: the spikes' follow camera, and `Camera_Update`
//!    (in the mode Player asked for during its update), which sets the letterbox's next target
//!    and the interface's alpha type (`Camera_UpdateInterface`).
//! 6. What `Play_Draw` changes: each actor's draw-time state (Player's foot IK writes into its
//!    joint table, so it's done once per game frame, not per rendered frame), the scene draw
//!    config (`Scene_Draw`: this frame's texture scrolls and colours), the view the next
//!    frame's target context reads, a second `Camera_Update` if the camera asked for one
//!    (`view.unk_124`), and what the reticle and the message box draw (`Attention_Draw` in
//!    `Interface_Draw`, then `Message_Draw`).
//! 7. Without a scene from the pack (the test course), the sandbox's void-out: below y −2000,
//!    Player respawns. In a scene Player's own exit and void checks do this.
//! 8. The render state of every actor and the camera is captured for blending.
//! 9. If the transition ended the scene, `Play_Init` for the next entrance replaces this play
//!    state (`PlayState::reinit`).
//!
//! While the game over menu is up (`IS_PAUSED`, `crate::kaleido`), steps 1's collision check and
//! 2 don't run, nor the cameras; its states run in `Message_Update`'s place. While a game over is
//! on otherwise (`crate::game_over`), `GameOver_Update` does. For four frames after an enemy's
//! finishing blow (`actorCtx.freezeFlashTimer`) the actors and cutscenes stop and the screen
//! flashes.
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
//!   XLU lists (`eng_gfx::DrawLists`), in `Actor_DrawAll` order, then into the overlay the
//!   HUD (`Interface_Draw`, once `Play_Init` has run `Interface_Init`) either side of the
//!   target reticle, and the message box (docs/adr/0017-interface-sprites.md).

use std::sync::Arc;

use eng_collision::bgcheck::CollisionContext;
use eng_gfx::DrawLists;
use eng_input::pad::{Input, PadMgr, PadState};
use eng_math::GAME_HZ;
use glam::{Mat4, Vec3};

use crate::actor::{ACTOR_FLAG_UPDATE_CULLING_DISABLED, ACTOR_FLAG_INSIDE_CULLING_VOLUME, Actor};
use crate::actor_ctx::{ACTORCAT_BG, ACTORCAT_MAX, ActorContext, ActorHandle, ActorImpl};
use crate::camera::{CAM_ID_MAIN, CAM_ID_NONE, CAM_ID_SUB_FIRST, CAM_STAT_ACTIVE, CAM_STAT_UNK100, CamFrame, CamView, CameraGlobals, CameraKind, FollowCamera, GameCamera, NUM_CAMS, PlayerView};
use crate::collision_check::{ColliderShape, CollisionCheckContext};
use crate::data::GameData;
use crate::interface::InterfaceContext;
use crate::letterbox::Letterbox;
use crate::message::{MessageContext, MessageTable, MsgFrame};
use crate::object_ctx::ObjectContext;
use crate::play_scene::{GameAssets, SceneState, TransitionState};
use crate::player_lib::PlayerRules;
use crate::room::RoomContext;
use crate::save::SaveContext;
use crate::scene::{ActorEntry, TransitionActorEntry};
use crate::spawn::{SceneFlags, Uninit};
use crate::target::{TargetCtx, TargetFrame};
use crate::transition::TRANS_MODE_OFF;

/// `PLAYER_STATE1_21`: climbing (a ladder or a vine wall).
pub const PLAYER_STATE1_21: u32 = 1 << 21;
/// `PLAYER_STATE1_27`: in water (swimming).
pub const PLAYER_STATE1_27: u32 = 1 << 27;

// `PlayerEnvHazard` (`player.h`).
pub const PLAYER_ENV_HAZARD_NONE: u8 = 0;
pub const PLAYER_ENV_HAZARD_HOTROOM: u8 = 1;
pub const PLAYER_ENV_HAZARD_UNDERWATER_FLOOR: u8 = 2;
pub const PLAYER_ENV_HAZARD_SWIMMING: u8 = 3;
pub const PLAYER_ENV_HAZARD_UNDERWATER_FREE: u8 = 4;

/// `VIEWPOINT_*` (`camera.h`): none, the locked bg camera (`BGCAM_INDEX_TOGGLE_LOCKED` + 1)
/// and the pivot one (`BGCAM_INDEX_TOGGLE_PIVOT` + 1).
pub const VIEWPOINT_NONE: u8 = 0;
pub const VIEWPOINT_LOCKED: u8 = 1;
pub const VIEWPOINT_PIVOT: u8 = 2;

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
    /// `play->billboardMtxF`: the view's rotation, inverted, so a mesh multiplied by it faces
    /// the camera (the lists that `gSPMatrix` segment 1).
    pub billboard: Mat4,
}

impl ViewInfo {
    /// From the view matrix: `billboardMtxF` is `view.viewing` without its translation,
    /// transposed.
    pub fn new(eye: Vec3, view: Mat4) -> ViewInfo {
        ViewInfo { eye, billboard: Mat4::from_mat3(glam::Mat3::from_mat4(view).transpose()) }
    }
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
    /// `shape.yOffset` (model units: `Actor_Draw` adds it times `scale.y`).
    pub y_offset: f32,
    /// Blended the short way round (binary angles).
    pub angles: Vec<i16>,
    /// Blended linearly.
    pub values: Vec<f32>,
    /// Switched halfway.
    pub switches: Vec<u32>,
    /// Don't blend into this state.
    pub teleported: bool,
    /// `colorFilterParams` and `colorFilterTimer` (`Actor_Draw`'s tint), as the frame left them.
    pub color_filter: (u16, u8),
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
            y_offset: a.shape_y_offset,
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
            y_offset: self.y_offset + (next.y_offset - self.y_offset) * t,
            joints: match (&self.joints, &next.joints) {
                (Some(a), Some(b)) => Some(a.lerp(b, t)),
                (_, b) => b.clone(),
            },
            // Extras of another shape (the actor was an `Uninit` last frame) aren't blended.
            angles: if self.angles.len() == next.angles.len() {
                self.angles.iter().zip(&next.angles).map(|(&a, &b)| lerp_angle(a, b, t)).collect()
            } else {
                next.angles.clone()
            },
            values: if self.values.len() == next.values.len() {
                self.values.iter().zip(&next.values).map(|(&a, &b)| a + (b - a) * t).collect()
            } else {
                next.values.clone()
            },
            switches: if t < 0.5 && self.switches.len() == next.switches.len() { self.switches.clone() } else { next.switches.clone() },
            teleported: false,
            color_filter: next.color_filter,
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
    /// The letterbox bars' height in rows of the 240-row frame (`Letterbox_GetSize`, blended).
    pub letterbox: f32,
    /// The view cut to this frame's (another camera became active, or the active one was put
    /// somewhere new at once: `GameCamera::view_cut`): it isn't blended from the last frame's.
    pub view_cut: bool,
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
            view: if next.view_cut {
                next.view
            } else {
                CamView {
                    eye: self.view.eye.lerp(next.view.eye, t),
                    at: self.view.at.lerp(next.view.at, t),
                    fov: self.view.fov + (next.view.fov - self.view.fov) * t,
                }
            },
            follow,
            letterbox: self.letterbox + (next.letterbox - self.letterbox) * t,
            view_cut: next.view_cut,
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
    /// `mainCamera` (`cameraPtrs[CAM_ID_MAIN]`).
    pub game_camera: GameCamera,
    /// `cameraPtrs[1..]`: the sub cameras (`subCameras`), made by `Play_CreateSubCamera`.
    pub sub_cameras: [Option<GameCamera>; NUM_CAMS - 1],
    /// `activeCamId`, `nextCamId`.
    pub active_cam_id: i16,
    pub next_cam_id: i16,
    /// Player while his own update has him out of the arena (docs/adr/0007-actor-ownership.md),
    /// as the camera sees him: set while his requests run, which in the C run inside his update
    /// with him in place (`OnePointCutscene_Init` with `&this->actor`, `Play_InitCameraDataUsingPlayer`).
    pub player_out_of_arena: Option<(PlayerView, crate::camera::CamActor)>,
    /// `z_camera.c`'s state every camera shares.
    pub cam_globals: CameraGlobals,
    /// The one-point cutscenes' statics (`crate::onepoint`): code segment statics, carried over
    /// scene changes.
    pub onepoint: crate::onepoint::OnePointStatics,
    /// `z_quake.c`'s request table (`crate::quake`): code segment statics, carried over scene
    /// changes.
    pub quake: crate::quake::QuakeStatics,
    /// `play->view`'s eye, at and fovy: what the active camera's last `Camera_Update` set
    /// (`View_LookAt`), which one-point cutscenes start from.
    pub view: crate::camera::CamView,
    /// `shrink_window.c`'s letterbox.
    pub letterbox: Letterbox,
    /// The spikes' follow camera, and which camera drives Player and the view.
    pub follow_camera: FollowCamera,
    /// The main camera starts as `Play_Init`'s does (floors' bg cameras on) even without the
    /// pack's scene: a custom level's.
    pub play_init_camera: bool,
    pub camera_kind: CameraKind,
    /// `actorCtx.targetCtx`.
    pub target_ctx: TargetCtx,
    /// `colChkCtx`: the colliders registered this frame.
    pub col_chk: CollisionCheckContext,
    /// The actor whose update (or draw) is running: the owner of the colliders it registers.
    pub cur_actor: Option<ActorHandle>,
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
    /// Link's object for his age (`Scene_CommandPlayerEntryList`).
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
    /// `R_SCENE_CAM_TYPE` (`SCENE_CAM_TYPE_*`, from the scene's `SCENE_CMD_ID_MISC_SETTINGS`).
    pub scene_cam_type: u8,
    /// `viewpoint` (`VIEWPOINT_*`): which of a fixed-camera scene's first two bg cameras is
    /// in use.
    pub viewpoint: u8,
    /// `bgCoverAlpha`: the vertical room-change planes' screen dimming.
    pub bg_cover_alpha: i16,
    /// `sfxSources` (`z_sfx_source.c`).
    pub sfx_sources: [crate::sfx_source::SfxSource; crate::sfx_source::NUM_SFX_SOURCES],
    /// How many scene changes led here (the renderer reloads its meshes when it changes).
    pub scene_changes: u32,
    /// `sRandInt` (`qrand.c`): the game's random numbers, shared by the actors.
    /// (Player keeps its own sequence, as the spikes did.)
    pub rand: Rand,
    /// `msgCtx`, and the messages it reads (the pack's, when the app has one).
    pub msg_ctx: MessageContext,
    pub messages: Option<Arc<MessageTable>>,
    /// `interfaceCtx`.
    pub interface_ctx: InterfaceContext,
    /// `z_map_exp.c`'s state (`crate::map`): `interfaceCtx`'s map fields, its REGs and statics.
    pub map: crate::map::MapState,
    /// `csCtx` (`crate::cutscene`), and `z_demo.c`'s statics.
    pub cs_ctx: crate::cutscene::CutsceneContext,
    pub demo: crate::cutscene::DemoStatics,
    /// `envFlags` (`CutsceneFlags_Set`): flags cutscenes set for actors.
    pub env_flags: [u16; 20],
    /// `cUpElfMsgs`: which of `sNaviQuestHintFiles` Navi's C-Up texts come from (the scene header's
    /// `SCENE_CMD_ID_SPECIAL_FILES`), `None` for none (`Play_InitScene`).
    pub c_up_elf_msgs: Option<usize>,
    /// `lightCtx`: the actors' point lights (`crate::lights`).
    pub light_ctx: crate::lights::LightContext,
    /// `envCtx` (`crate::env::EnvCtx`) and `z_kankyo.c`'s statics that outlive a scene
    /// (`gWeatherMode`, the lightning).
    pub env_ctx: crate::env::EnvCtx,
    pub env_statics: crate::env::EnvStatics,
    /// The lightning bolts `Environment_DrawLightning` draws this frame, and the lightning's
    /// flash (`Environment_DrawLightningFlash`: its colour and alpha), for the renderer.
    pub lightning_bolts: Vec<crate::env::LightningBolt>,
    pub lightning_flash: Option<[u8; 4]>,
    /// `Environment_DrawRain`'s drops and rings this frame (`crate::weather`).
    pub rain: crate::weather::RainDraw,
    /// `gVisMonoColor` (`z_play.c`): the screen's monochrome tint (`VisMono`) a cutscene sets.
    pub vis_mono_color: [u8; 4],
    /// `haltAllActors`: the actors frozen (`Actor_UpdateAll` skipped).
    pub halt_all_actors: bool,
    /// `pauseCtx`: the game over menu's stand-in (`crate::kaleido`; the pause menu itself isn't
    /// ported).
    pub pause_ctx: crate::kaleido::PauseContext,
    /// `gameOverCtx` (`crate::game_over`), and `z_game_over.c`'s `sGameOverTimer`.
    pub game_over_ctx: crate::game_over::GameOverContext,
    pub game_over_timer: i16,
    /// `actorCtx.titleCtx`: the place name's title card.
    pub title_ctx: crate::title_card::TitleCardContext,
    /// The game's side of the audio (`crate::audio`): its statics are the code segment's, so a
    /// scene change carries them over. A driver hands its `GameOp`s to the audio side after
    /// each frame and gives it the audio side's `AudioView` before the next
    /// (docs/adr/0026-the-games-audio.md).
    pub audio: crate::audio::GameAudio,
    /// Not in the C: an audio side this play state hands each frame to itself, after
    /// `Audio_Update` (headless runs and tests; the window drives its device through
    /// `advance_with`). A scene change carries it over.
    pub audio_side: Option<Box<dyn crate::audio::AudioSide>>,
    /// `sequenceCtx`: the scene's music and ambience (`Scene_CommandSoundSettings`).
    pub sequence_ctx: crate::audio::scene::SceneSequences,
    /// `envCtx.timeSeqState` (`Environment_PlayTimeBasedSequence`).
    pub time_seq_state: u8,
    /// The children each running init spawned (`Actor_SpawnAsChild`), innermost last: they get
    /// the parent's handle once it's in the actor context.
    pub(crate) init_children: Vec<Vec<ActorHandle>>,
    /// The ported overlays' file-scope statics that instances share (their `.bss`), by
    /// `ACTOR_*` id (`overlay_static`). A play state starts without any, as `Play_Init`'s
    /// fresh overlay loads zero theirs. (An overlay unloads, and its statics reset, when its
    /// last actor goes mid-scene: that isn't modelled.)
    pub overlay_statics: std::collections::HashMap<i16, Box<dyn std::any::Any>>,
    pub(crate) next_play_init: bool,
    /// Whether a game frame has run since this state was made. `Play_Main` always runs
    /// `Play_Update` before its first `Play_Draw`, but the frontends draw at the display rate
    /// and can present a new state before its first tick.
    pub(crate) updated: bool,
    /// What covers the screen until then, when a transition is due (`screen_fill`): the fill
    /// the previous state ended on (its finished fade-out), or black for a first `Play_Init`.
    pub(crate) pre_update_fill: Option<[u8; 4]>,
    acc: f32,
    prev: Option<RenderFrame>,
    cur: Option<RenderFrame>,
    /// The camera `cur` was captured from.
    captured_cam_id: i16,
    /// `sEffectSsInfo`: the soft sprites (`crate::effect`), a fresh table each `Play_Init`.
    pub effect_ss: crate::effect::EffectSsInfo,
    /// Sounds a spawn's reuse of a slot stopped (`EffectSs_Delete`), for `Audio_StopSfxByPos`.
    pub effect_ss_stops: Vec<crate::audio::sfx::SfxPos>,
    /// `sEffectContext`: the sparks and the shield particles.
    pub effect_ctx: crate::effect::EffectContext,
    /// This game frame's effect draws (`Effect_DrawAll`, `EffectSs_DrawAll`), appended after the
    /// actors' by `draw`.
    pub effect_draws: crate::effect::EffectDraws,
    /// `play->state.frames`: the game state's frames run (`GameState_Update` counts them after
    /// each one).
    pub state_frames: u32,
}

impl PlayState {
    /// An empty play state over `col`. Spawn Player (and the rest) with the content crate,
    /// then call `reset_blending`.
    pub fn new(data: Arc<GameData>, rules: Arc<PlayerRules>, col: CollisionContext, spawn: (Vec3, i16), adult: bool) -> PlayState {
        let pv = PlayerView { pos: spawn.0, shape_yaw: spawn.1, shape_pitch: 0, world_yaw: spawn.1, adult, run_speed_limit: data.regs[if adult { 0 } else { 1 }].reg(45), gravity: 0.0, climbing: false, state1: 0, iron_boots: false };
        let game_camera = GameCamera::new(&data.camera, &pv);
        let view = crate::camera::CamView { eye: game_camera.eye, at: game_camera.at, fov: game_camera.fov };
        PlayState {
            follow_camera: FollowCamera::behind(spawn.0, spawn.1, adult),
            play_init_camera: false,
            game_camera,
            sub_cameras: [None, None, None],
            active_cam_id: CAM_ID_MAIN,
            player_out_of_arena: None,
            next_cam_id: CAM_ID_MAIN,
            cam_globals: CameraGlobals::main_init(),
            onepoint: crate::onepoint::OnePointStatics::new(&data.camera.onepoint),
            quake: Default::default(),
            view,
            letterbox: Letterbox::new(),
            camera_kind: CameraKind::Game,
            data,
            rules,
            col,
            actors: ActorContext::default(),
            player: None,
            target_ctx: TargetCtx::new(),
            col_chk: CollisionCheckContext::default(),
            cur_actor: None,
            gameplay_frames: 0,
            input: Input::default(),
            pad: PadMgr::default(),
            debug: Debug { foot_ik: true, placeholders: false },
            spawn,
            respawn_player: None,
            view_proj: Mat4::IDENTITY,
            // The spikes' view of a scene (no Play_Init) plays on the map select's file, whose
            // Link has the sword and shield (docs/adr/0019-inventory-and-saves.md).
            save: SaveContext::debug(0, adult, crate::env::clock_time(10, 0) as u16),
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
            scene_cam_type: crate::scene::SCENE_CAM_TYPE_DEFAULT,
            viewpoint: VIEWPOINT_NONE,
            bg_cover_alpha: 0,
            sfx_sources: Default::default(),
            scene_changes: 0,
            rand: Rand::default(),
            msg_ctx: MessageContext::new(),
            cs_ctx: Default::default(),
            demo: Default::default(),
            env_flags: [0; 20],
            c_up_elf_msgs: None,
            light_ctx: Default::default(),
            env_ctx: crate::env::EnvCtx::init(crate::env::LIGHT_MODE_TIME, 0, 0, 0),
            env_statics: Default::default(),
            lightning_bolts: Vec::new(),
            lightning_flash: None,
            rain: Default::default(),
            vis_mono_color: [0; 4],
            halt_all_actors: false,
            pause_ctx: Default::default(),
            game_over_ctx: Default::default(),
            game_over_timer: 0,
            title_ctx: Default::default(),
            audio: Default::default(),
            audio_side: None,
            sequence_ctx: Default::default(),
            time_seq_state: crate::audio::scene::TIMESEQ_DISABLED,
            init_children: Vec::new(),
            overlay_statics: Default::default(),
            messages: None,
            interface_ctx: InterfaceContext::default(),
            map: Default::default(),
            next_play_init: false,
            updated: false,
            pre_update_fill: Some([0, 0, 0, 255]),
            acc: 0.0,
            prev: None,
            cur: None,
            captured_cam_id: CAM_ID_MAIN,
            effect_ss: Default::default(),
            effect_ss_stops: Vec::new(),
            effect_ctx: Default::default(),
            effect_draws: Default::default(),
            state_frames: 0,
        }
    }

    /// The statics of overlay `id` (see `overlay_statics`), zeroed (`Default`) on first use.
    pub fn overlay_static<T: Default + 'static>(&mut self, id: i16) -> &mut T {
        self.overlay_statics.entry(id).or_insert_with(|| Box::new(T::default())).downcast_mut::<T>().expect("one statics type per overlay")
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

    /// The player's view for the camera (`Camera_InitDataUsingPlayer`, `Camera_Update`).
    pub fn player_view(&self) -> Option<PlayerView> {
        let h = self.player?;
        match self.actors.get(h) {
            Some(p) => self.player_view_of_impl(p),
            None => self.player_out_of_arena.as_ref().map(|(pv, _)| *pv),
        }
    }

    /// `player_view` for Player given as himself (out of the arena in his own update).
    pub fn player_view_of_impl(&self, p: &dyn ActorImpl) -> Option<PlayerView> {
        let pi = p.as_player()?;
        let adult = pi.adult();
        let a = p.base();
        Some(PlayerView {
            pos: a.world_pos,
            shape_yaw: a.shape_rot.y,
            shape_pitch: a.shape_rot.x,
            world_yaw: a.world_rot.y,
            adult,
            run_speed_limit: self.data.regs[if adult { 0 } else { 1 }].reg(45),
            gravity: a.gravity,
            climbing: pi.state_flags1() & PLAYER_STATE1_21 != 0,
            state1: pi.state_flags1(),
            iron_boots: pi.current_boots() == crate::actor_ctx::PLAYER_BOOTS_IRON,
        })
    }

    /// Puts both cameras behind Player (`Camera_Init` and the follow camera's start). In a
    /// scene entered by `Play_Init`, the main camera also gets `Play_Init`'s flags and the
    /// room's setting (`GameCamera::play_init_settings`).
    pub fn reset_cameras(&mut self) {
        if let Some(pv) = self.player_view() {
            self.follow_camera = FollowCamera::behind(pv.pos, pv.shape_yaw, pv.adult);
            self.game_camera = GameCamera::new(&self.data.camera, &pv);
            if self.scene.is_some() && (self.assets.is_some() || self.play_init_camera) {
                let room = self.cam_room();
                self.game_camera.play_init_settings(room);
            }
        }
        self.view_proj = self.camera_view_proj();
    }

    /// `Camera_GetCamDirYaw` of the active camera: where it looks.
    pub fn cam_dir_yaw(&self) -> i16 {
        match self.camera_kind {
            CameraKind::Game => self.active_camera().cam_dir[1],
            CameraKind::Follow => self.follow_camera.input_dir_yaw(),
        }
    }

    /// `Camera_GetInputDirYaw` of the active camera.
    pub fn input_dir_yaw(&self) -> i16 {
        match self.camera_kind {
            CameraKind::Game => self.active_camera().input_dir_yaw(),
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

    /// One actor's turn in `Actor_UpdateAll`. `can_freeze_category`: Player's state freezes this
    /// actor's category (`sCategoryFreezeMasks`), and `exempt` the actors it doesn't freeze.
    fn update_actor(&mut self, h: ActorHandle, player_pos: Option<Vec3>, can_freeze_category: bool, exempt: &[Option<ActorHandle>]) {
        let Some(mut a) = self.actors.take(h) else { return };
        let base = a.base_mut();
        if base.world_pos.y < -25000.0 {
            base.world_pos.y = -25000.0;
        }
        base.sfx = 0;
        if let Some(u) = a.as_any_mut().downcast_mut::<Uninit>() {
            // actor->init != NULL: initialise once the object is loaded, and skip this frame.
            if u.actor.obj_bank_index.is_some_and(|b| self.object_ctx.is_loaded(b)) {
                let Ok(u) = a.into_any().downcast::<Uninit>() else { unreachable!() };
                let init = self.run_init(h, u.actor, u.ctor);
                self.actors.put_back(h, init);
            } else {
                self.actors.put_back(h, a);
            }
            return;
        }
        if base_bank_dropped(a.base(), &self.object_ctx) {
            a.base_mut().kill();
        } else if can_freeze_category && !exempt.contains(&Some(h)) {
            // Frozen by Player's state (talking, dead, ...): only the damage is reset. (The
            // ocarina's exception, ACTOR_FLAG_UPDATE_DURING_OCARINA, never comes up.)
            a.base_mut().col_chk_info.reset_damage();
            self.actors.put_back(h, a);
            return;
        }
        let base = a.base_mut();
        if base.killed {
            // update == NULL: Actor_Delete (isDrawn isn't tracked, so it goes at once): its
            // sounds stop (Audio_StopSfxByPos(&actor->projectedPos)), then Actor_Destroy.
            self.audio.stop_sfx_by_pos(crate::audio::sfx::SfxPos::Actor(h));
            a.destroy(self);
            self.actors.put_back(h, a);
            self.actor_remove_from_category(h);
            if self.player == Some(h) {
                self.player = None;
            }
            return;
        }
        base.prev_pos = base.world_pos;
        if let Some(pp) = player_pos {
            base.update_distances(pp);
        }
        base.flags &= !crate::actor::ACTOR_FLAG_SFX_FOR_PLAYER_BODY_HIT;
        // Culling (Actor_CullingVolumeTest, which sets ACTOR_FLAG_INSIDE_CULLING_VOLUME for actors in view) isn't ported:
        // every actor counts as in view.
        base.flags |= ACTOR_FLAG_INSIDE_CULLING_VOLUME;
        let due = if base.freeze_timer > 0 {
            base.freeze_timer -= 1;
            base.freeze_timer == 0
        } else {
            true
        };
        if due && base.flags & (ACTOR_FLAG_UPDATE_CULLING_DISABLED | ACTOR_FLAG_INSIDE_CULLING_VOLUME) != 0 {
            let target = self.player_target();
            base.is_targeted = target == Some(h);
            if base.target_priority != 0 && target.is_none() {
                base.target_priority = 0;
            }
            if base.color_filter_timer != 0 {
                base.color_filter_timer -= 1;
            }
            self.cur_actor = Some(h);
            a.update(self);
            self.cur_actor = None;
            // DynaPoly_UnsetAllInteractFlags.
            if let Some(bg) = a.dyna_bg_id() {
                self.col.dyna.unset_all_interact_flags(bg);
            }
        }
        a.base_mut().col_chk_info.reset_damage();
        self.actors.put_back(h, a);
    }

    /// `CollisionCheck_SetAT` for the running actor's collider `id` (`owner` is its base).
    pub fn collision_check_set_at(&mut self, owner: &Actor, id: u8, c: &mut impl ColliderShape) -> i32 {
        self.col_chk.set_at(self.cur_actor, owner, id, c)
    }

    /// `CollisionCheck_SetAC`.
    pub fn collision_check_set_ac(&mut self, owner: &Actor, id: u8, c: &mut impl ColliderShape) -> i32 {
        self.col_chk.set_ac(self.cur_actor, owner, id, c)
    }

    /// `CollisionCheck_SetOC`.
    pub fn collision_check_set_oc(&mut self, owner: &Actor, id: u8, c: &mut impl ColliderShape) -> i32 {
        self.col_chk.set_oc(self.cur_actor, owner, id, c)
    }

    /// Player's `focusActor`.
    pub fn player_target(&self) -> Option<ActorHandle> {
        self.actors.get(self.player?)?.as_player()?.target()
    }

    /// One game frame with an explicit input (scripted runs and tests).
    pub fn tick_with(&mut self, input: Input) {
        self.audio.frames += 1;
        self.update_transition();
        self.object_ctx.update_bank();
        self.input = input;
        // Play_Update: KaleidoSetup_Update, only with no message box and no game over
        // (gameMode GAMEMODE_NORMAL), before the actors.
        if self.msg_ctx.msg_mode == crate::message::MSGMODE_NONE && self.game_over_ctx.state == crate::game_over::GAMEOVER_INACTIVE {
            self.kaleido_setup_update();
        }
        let is_paused = self.pause_ctx.is_paused();
        if !is_paused {
            self.gameplay_frames += 1;
            // (Rumble_SetUpdateEnabled.) An enemy's finishing blow (Enemy_StartFinishingBlow)
            // stops the frame for four frames, flashing the screen on the odd ones.
            let flash = self.actors.freeze_flash_timer;
            if flash != 0 {
                self.actors.freeze_flash_timer -= 1;
            }
            if flash != 0 && flash < 5 {
                let t = self.actors.freeze_flash_timer;
                self.transition.screen_fill = (t > 0 && t % 2 != 0).then_some([150, 150, 150, 80]);
            } else {
                self.room_finish_load();
                let hit_fx = self.col_chk.check(&mut self.actors);
                self.collision_check_hit_fx(hit_fx);
                self.col_chk.clear();
                if !self.halt_all_actors {
                    self.update_all_actors();
                }
                // The cutscene system (z_demo.c): Cutscene_UpdateManual, then Cutscene_UpdateScripted.
                self.update_manual();
                self.update_scripted();
                // The effects (crate::effect): Effect_UpdateAll, then EffectSs_UpdateAll.
                self.effect_update_all();
                self.effect_ss_update_all();
            }
        }
        // (func_80095AA0 for both rooms: no room behaviour is ported.) The viewpoint.
        self.update_viewpoint();
        // The game over menu (KaleidoScopeCall_Update) while paused, else GameOver_Update during
        // a game over, else Message_Update; then Interface_Update.
        if self.pause_ctx.is_paused() {
            self.kaleido_scope_call_update();
        } else if self.game_over_ctx.state != crate::game_over::GAMEOVER_INACTIVE {
            self.game_over_update();
        } else {
            self.with_msg(|m, f| m.update(f));
        }
        self.interface_update();
        // AnimTaskQueue_Update: every actor's queued animation requests.
        for h in self.actors.all() {
            if let Some(a) = self.actors.get_mut(h) {
                a.animation_update();
            }
        }
        // SfxSource_UpdateAll, then Letterbox_Update(R_UPDATE_RATE), then the cameras (they
        // follow Player).
        self.sfx_source_update_all();
        self.letterbox.update(3);
        if !is_paused {
            if let Some(p) = self.player.and_then(|ph| self.actors.get(ph)) {
                let (pos, facing, speed) = (p.base().world_pos, p.base().shape_rot.y, p.as_player().map(|i| i.speed_xz()).unwrap_or(0.0));
                self.follow_camera.update(&input, pos, facing, speed);
            }
            self.camera_update(input);
        }
        // Environment_Update (pauseCtx.state 0: no pause menu): the rain, the time of day's
        // music, the lights (time doesn't pass).
        if self.assets.is_some() {
            self.env_ctx.update_rain(self.gameplay_frames);
            self.environment_play_time_based_sequence();
            self.environment_update_lights();
        }
        // Play_Draw's environment, before the rooms and the actors: the lightning's strike and
        // its bolts (Environment_UpdateLightningStrike, Environment_DrawLightning).
        if self.assets.is_some() {
            self.environment_update_lightning_strike();
            let (eye, at) = (self.view.eye, self.view.at);
            self.lightning_bolts = self.env_statics.update_lightning_bolts(eye, at, &mut self.rand);
            // After the rooms and the skybox: Environment_DrawRain.
            self.rain = self.environment_draw_rain();
        }
        // Play_Draw: the actors' draw-time state (Player's foot IK), and the view it sets up
        // (play->viewProjectionMtxF), which the next frame's target context reads.
        for h in self.actors.all() {
            if let Some(mut a) = self.actors.take(h) {
                self.cur_actor = Some(h);
                a.draw_update(self);
                self.cur_actor = None;
                self.actors.put_back(h, a);
            }
        }
        self.flush_effect_ss_stops();
        // The end of Actor_DrawAll: Effect_DrawAll, then EffectSs_DrawAll.
        self.effect_draw_all();
        let frames = self.gameplay_frames;
        let tree_dead = self.save.get_event_chk_inf(crate::save::EVENTCHKINF_07);
        if let Some(s) = &mut self.scene {
            s.draw.event_chk_inf_07 = tree_dead;
            s.run_draw_config(frames);
        }
        // The end of Play_Draw: a camera that asked for it (view.unk_124) updates again.
        if self.game_camera.view_unk_124 != 0 {
            // Camera_Update(GET_ACTIVE_CAM(this)).
            self.update_camera(self.active_cam_id, input);
            self.game_camera.view_unk_124 = 0;
        }
        // Camera_Finish(GET_ACTIVE_CAM(this)): a one-point cutscene's camera whose timer ran out.
        self.camera_finish(self.active_cam_id);
        self.view_proj = self.camera_view_proj();
        // Actor_DrawAll: each actor's projectedPos through the frame's
        // viewProjectionMtxF, then the sound it asked for (Actor_UpdateFlaggedAudio).
        self.actor_draw_all_sfx();
        // Interface_Draw: the Z-target reticle (Attention_Draw) with this frame's view.
        let reticle_player = self.player.and_then(|h| self.actors.get(h)).and_then(|p| p.as_player()).map(|pi| crate::target::ReticlePlayer { state1_6: pi.state_flags1() & (1 << 6) != 0, target: pi.target() });
        if let Some(rp) = reticle_player {
            crate::target::draw_update(&mut self.target_ctx, &self.actors, self.view_proj, rp);
        }
        // Play_DrawOverlayElements: the pause menu's draw (the game over prompt's stick), then
        // Message_Draw, then GameOver_FadeInLights.
        self.kaleido_scope_draw_update();
        self.with_msg(|m, f| m.draw_update(f));
        if self.game_over_ctx.state != crate::game_over::GAMEOVER_INACTIVE {
            self.game_over_fade_in_lights();
        }
        // The sandbox's void-out (a scene from the pack has Player's own).
        if self.assets.is_none() && self.player.and_then(|ph| self.actors.actor(ph)).is_some_and(|a| a.world_pos.y < -2000.0) {
            self.respawn();
            return;
        }
        self.prev = self.cur.take();
        self.cur = Some(self.capture());
        self.updated = true;
        // GameState_Update: gameState->frames++.
        self.state_frames = self.state_frames.wrapping_add(1);
        // Graph_Update: Audio_Update once the game state's frame is done.
        self.audio_update();
        if self.next_play_init {
            // GameState_Destroy: AudioMgr_StopAllSfx, Audio_Update again, then the next
            // Play_Init.
            self.audio.audio_mgr_stop_all_sfx();
            self.audio_update();
            self.reinit();
        }
        if let Some(side) = &mut self.audio_side {
            side.hand_over(&mut self.audio);
        }
    }

    /// `Audio_Update` with the sound effects' positions: an actor's `projectedPos`.
    fn audio_update(&mut self) {
        use crate::audio::sfx::SfxPos;
        let (actors, sources, effects) = (&self.actors, &self.sfx_sources, &self.effect_ss);
        self.audio.audio_update_with(&|p| match p {
            SfxPos::Default => Some(Vec3::ZERO),
            SfxPos::Actor(h) => actors.actor(h).map(|a| a.projected_pos),
            SfxPos::Source(i) => sources.get(i as usize).map(|s| s.projected_pos),
            SfxPos::EffectSsPos(i) => effects.table.get(i as usize).map(|e| e.pos),
            SfxPos::EffectSsVec(i) => effects.table.get(i as usize).map(|e| e.vec),
        });
    }

    /// `Actor_DrawAll`'s sound part (`Actor_DrawAll`): every actor's `projectedPos` and
    /// `projectedW` (`SkinMatrix_Vec3fMtxFMultXYZW` on `viewProjectionMtxF`), and the sound in
    /// its `sfx` (`Actor_UpdateFlaggedAudio`).
    fn actor_draw_all_sfx(&mut self) {
        use crate::actor::{ACTOR_FLAG_SFX_ACTOR_POS_2, ACTOR_AUDIO_FLAG_SFX_CENTERED_1, ACTOR_AUDIO_FLAG_SFX_CENTERED_2, ACTOR_FLAG_SFX_TIMER};
        use crate::audio::sfx::{SfxF32, SfxPos, SfxS8, SFX_FLAG};
        let vp = self.view_proj;
        for h in self.actors.all() {
            let Some(a) = self.actors.actor_mut(h) else { continue };
            let c = vp * a.world_pos.extend(1.0);
            a.projected_pos = c.truncate();
            a.projected_w = c.w;
            let (sfx, flags) = (a.sfx, a.flags);
            if sfx != 0 {
                // Actor_UpdateFlaggedAudio.
                let au = &mut self.audio;
                if flags & ACTOR_FLAG_SFX_ACTOR_POS_2 != 0 {
                    au.play_sfx_general(sfx, SfxPos::Actor(h), 4, SfxF32::One, SfxF32::One, SfxS8::Zero);
                } else if flags & ACTOR_AUDIO_FLAG_SFX_CENTERED_1 != 0 {
                    au.play_sfx_centered(sfx);
                } else if flags & ACTOR_AUDIO_FLAG_SFX_CENTERED_2 != 0 {
                    au.play_sfx_centered2(sfx);
                } else if flags & ACTOR_FLAG_SFX_TIMER != 0 {
                    au.func_800f4c58(SfxPos::Default, crate::audio::sfx::NA_SE_SY_TIMER - SFX_FLAG, sfx.wrapping_sub(1) as i8 as u8);
                } else {
                    au.play_sfx_at_pos(SfxPos::Actor(h), sfx);
                }
            }
            // The actor's draw (an actor in the arena is drawn: init done, culling not ported).
            if let Some(mut a) = self.actors.take(h) {
                self.cur_actor = Some(h);
                a.draw_sfx(self);
                self.cur_actor = None;
                self.actors.put_back(h, a);
            }
        }
    }

    /// `KaleidoSetup_Update` (`z_kaleido_setup.c`): Start opens the pause menu when nothing
    /// stops it (no menu open, no transition, no cutscene: `Play_InCsMode`; the shooting
    /// gallery, magic filling and the bowling alley's switch don't come up). The pause menu
    /// isn't ported: its equipping's stand-in (`pause_menu_equip`) runs in its place, at once
    /// (docs/adr/0021-mido-the-shop-and-the-pause-stand-in.md). L with C-Up is the debug menu,
    /// which needs `BREG(0)`: nothing.
    fn kaleido_setup_update(&mut self) {
        use crate::transition::{TRANS_MODE_OFF, TRANS_TRIGGER_OFF};
        use eng_input::pad::{BTN_CUP, BTN_L, BTN_START};
        if self.transition.trigger != TRANS_TRIGGER_OFF || self.transition.mode != TRANS_MODE_OFF || self.play_in_cs_mode() {
            return;
        }
        if self.input.cur.held(BTN_L) && self.input.press.held(BTN_CUP) {
            return;
        }
        if self.input.press.held(BTN_START) && self.pause_menu_equip() {
            log::info!("equipped (the pause menu's stand-in): equipment {:#06x}, B {:#04x}, C-Left {:#04x}", self.save.equips.equipment, self.save.equips.button_items[0], self.save.equips.button_items[1]);
        }
    }

    /// The pause menu's equipping, as a stand-in (docs/adr/0019-inventory-and-saves.md):
    /// `SaveContext::equip_owned_unworn` and `SaveContext::equip_sticks_on_empty_c_left`, then
    /// `Player_SetEquipmentData` as the menu's closing runs it. Returns whether anything was
    /// equipped.
    pub fn pause_menu_equip(&mut self) -> bool {
        let equipment = self.save.equip_owned_unworn();
        let sticks = self.save.equip_sticks_on_empty_c_left();
        if !equipment && !sticks {
            return false;
        }
        let (data, save) = (self.data.clone(), self.save.clone());
        if let Some(p) = self.player.and_then(|h| self.actors.get_mut(h)).and_then(|p| p.as_player_mut()) {
            p.set_equipment_data(&data, &save);
        }
        true
    }

    /// `Play_Update`'s cameras: every camera but the active one (`nextCamId`), then that one,
    /// each through `Camera_Update` (a waiting or cut camera does little).
    fn camera_update(&mut self, input: Input) {
        self.next_cam_id = self.active_cam_id;
        let next = self.next_cam_id;
        for i in 0..NUM_CAMS as i16 {
            if i != next && self.camera(i).is_some() {
                self.update_camera(i, input);
            }
        }
        self.update_camera(next, input);
    }

    /// `Camera_Update` for camera `id`, following Player.
    fn update_camera(&mut self, id: i16, input: Input) {
        let Some(pv) = self.player_view() else { return };
        let Some(cam) = self.camera(id) else { return };
        let (target, door_actor) = (cam.target, cam.door_params.door_actor);
        // Actor_GetFocus(camera->target), unless it was killed (update == NULL).
        let target_focus = target.and_then(|h| self.actors.actor(h)).filter(|a| !a.killed).map(|a| a.focus_pos);
        let door = door_actor.and_then(|h| self.actors.actor(h)).map(|a| (a.world_pos, [a.shape_rot.x, a.shape_rot.y, a.shape_rot.z]));
        let target_pos_rot = target.and_then(|h| self.actors.actor(h)).filter(|a| !a.killed).map(|a| (a.world_pos, [a.shape_rot.x, a.shape_rot.y, a.shape_rot.z]));
        let target_info = target.and_then(|h| self.cam_actor(h));
        let player_actor_info = self.player.and_then(|h| self.cam_actor(h));
        let main_player_pos_rot = self.game_camera.main_player_pos_rot();
        let player_waist = self.player.and_then(|h| self.actors.get(h)).and_then(|p| p.as_player()).map(|p| p.body_part(crate::actor_ctx::PLAYER_BODYPART_WAIST)).unwrap_or(pv.pos);
        let player_melee_weapon_active = self.player.and_then(|h| self.actors.get(h)).and_then(|p| p.as_player()).is_some_and(|p| p.melee_weapon_state() != 0);
        let oc_lines = self.col_chk.oc_lines(&mut self.actors);
        let cameras = std::array::from_fn(|i| self.camera(i as i16).map(|c| (c.eye, c.at)));
        let f = CamFrame {
            col: &self.col,
            player: pv,
            target_focus,
            door,
            transitioning: self.transition.mode != TRANS_MODE_OFF,
            frames: self.gameplay_frames,
            input,
            player_actor: self.player,
            oc_lines: &oc_lines,
            target_pos_rot,
            cs_active: self.cs_ctx.state != crate::cutscene::CS_STATE_IDLE,
            target: target_info,
            player_actor_info,
            view: self.view,
            main_player_pos_rot,
            player_waist,
            player_melee_weapon_active,
            health: self.save.health,
            skybox_disabled: self.scene.as_ref().and_then(|s| s.room(self.room_ctx.cur.num)).is_some_and(|r| r.skybox_disabled),
            cameras,
        };
        let cam = if id == CAM_ID_MAIN { Some(&mut self.game_camera) } else { self.sub_cameras.get_mut((id - CAM_ID_SUB_FIRST) as usize).and_then(|c| c.as_mut()) };
        let Some(cam) = cam else { return };
        cam.update(&self.data.camera, &f, &mut self.letterbox, &mut self.cam_globals, &mut self.onepoint, &mut self.rand, &mut self.quake);
        // View_LookAt: an active camera's update sets play->view (with the quakes' shake).
        if cam.status == crate::camera::CAM_STAT_ACTIVE {
            self.view = cam.shaken_view();
        }
        // Camera_Subj4 moves Player (camera->player->actor.world.pos, shape.rot.y).
        let write = cam.player_write.take();
        // Camera_UpdateInterface's Interface_ChangeHudVisibilityMode.
        let alpha = cam.interface_alpha_change.take();
        if let Some((pos, yaw)) = write
            && let Some(a) = self.player.and_then(|h| self.actors.actor_mut(h))
        {
            a.world_pos = pos;
            a.shape_rot.y = yaw;
        }
        if let Some(alpha_type) = alpha {
            crate::interface::change_alpha(&mut self.save, alpha_type);
        }
        self.camera_sfx();
        self.apply_cam_requests(id);
    }

    /// The sounds the cameras asked for (`crate::camera::CamSfx`), played in order as the C
    /// plays them from inside the camera code.
    pub fn camera_sfx(&mut self) {
        use crate::audio::sfx::{NA_SE_SY_ATTENTION_ON, NA_SE_SY_ATTENTION_URGENCY, NA_SE_SY_ERROR, SfxPos};
        use crate::camera::CamSfx;
        let mut all = std::mem::take(&mut self.game_camera.sfx);
        for c in self.sub_cameras.iter_mut().flatten() {
            all.append(&mut c.sfx);
        }
        for s in all {
            match s {
                CamSfx::ModeChange(1) => self.audio.play_sfx_centered(0),
                // ROOM_TYPE_DUNGEON: a dungeon room.
                CamSfx::ModeChange(2) => self.audio.play_sfx_centered(if self.room_ctx.cur.behavior_type1 == 1 { NA_SE_SY_ATTENTION_URGENCY } else { NA_SE_SY_ATTENTION_ON }),
                CamSfx::ModeChange(4) => self.audio.play_sfx_centered(NA_SE_SY_ATTENTION_URGENCY),
                CamSfx::ModeChange(8) => self.audio.play_sfx_centered(NA_SE_SY_ATTENTION_ON),
                CamSfx::ModeChange(_) => {}
                CamSfx::Error => self.audio.play_sfx_centered(NA_SE_SY_ERROR),
                CamSfx::Sfx(id) => self.audio.play_sfx_centered(id),
                CamSfx::Crawl => {
                    let Some(ph) = self.player else { continue };
                    let unk_89e = self.actors.get(ph).and_then(|p| p.as_player()).map(|p| p.unk_89e()).unwrap_or(0);
                    self.audio.func_800f4010(SfxPos::Actor(ph), unk_89e.wrapping_add(0x8B0), 4.0);
                }
            }
        }
    }

    /// `cameraPtrs[id]` (`CAM_ID_NONE`: the active one).
    pub fn camera(&self, id: i16) -> Option<&GameCamera> {
        let id = if id == CAM_ID_NONE { self.active_cam_id } else { id };
        if id == CAM_ID_MAIN { Some(&self.game_camera) } else { self.sub_cameras.get(usize::try_from(id - CAM_ID_SUB_FIRST).ok()?)?.as_ref() }
    }

    pub fn camera_mut(&mut self, id: i16) -> Option<&mut GameCamera> {
        let id = if id == CAM_ID_NONE { self.active_cam_id } else { id };
        if id == CAM_ID_MAIN { Some(&mut self.game_camera) } else { self.sub_cameras.get_mut(usize::try_from(id - CAM_ID_SUB_FIRST).ok()?)?.as_mut() }
    }

    /// `GET_ACTIVE_CAM(play)`.
    pub fn active_camera(&self) -> &GameCamera {
        self.camera(self.active_cam_id).unwrap_or(&self.game_camera)
    }

    /// `Play_CreateSubCamera`: the first free sub camera, through `Camera_Init`; `CAM_ID_NONE`
    /// when all three are taken.
    pub fn create_sub_camera(&mut self) -> i16 {
        let Some(i) = self.sub_cameras.iter().position(|c| c.is_none()) else {
            log::error!("camera control: error: fulled sub camera system area");
            return CAM_ID_NONE;
        };
        let id = i as i16 + CAM_ID_SUB_FIRST;
        self.sub_cameras[i] = Some(GameCamera::init_sub(&self.data.camera, &mut self.cam_globals, id));
        id
    }

    /// `Play_ChangeCameraStatus`: `CAM_STAT_ACTIVE` also makes it the active camera.
    pub fn change_camera_status(&mut self, id: i16, status: i16) -> i16 {
        let id = if id == CAM_ID_NONE { self.active_cam_id } else { id };
        if status == CAM_STAT_ACTIVE {
            self.active_cam_id = id;
        }
        self.camera_mut(id).map(|c| c.change_status(status)).unwrap_or(0)
    }

    /// `Play_ClearCamera`.
    pub fn clear_camera(&mut self, id: i16) {
        let id = if id == CAM_ID_NONE { self.active_cam_id } else { id };
        if id == CAM_ID_MAIN {
            log::error!("camera control: error: never clear camera !!");
            return;
        }
        match self.sub_cameras.get_mut((id - CAM_ID_SUB_FIRST) as usize) {
            Some(c @ Some(_)) => {
                if let Some(cam) = c.as_mut() {
                    cam.change_status(CAM_STAT_UNK100);
                }
                *c = None;
            }
            _ => log::error!("camera control: error: camera No.{id} already cleared"),
        }
    }

    /// `Play_ClearAllSubCameras`.
    pub fn clear_all_sub_cameras(&mut self) {
        for id in CAM_ID_SUB_FIRST..NUM_CAMS as i16 {
            if self.camera(id).is_some() {
                self.clear_camera(id);
            }
        }
        self.active_cam_id = CAM_ID_MAIN;
    }

    /// `Play_RequestCameraSetting`.
    pub fn camera_change_setting(&mut self, id: i16, setting: i16) -> i16 {
        let d = self.data.clone();
        self.camera_mut(id).map(|c| c.change_setting(&d.camera, setting)).unwrap_or(-99)
    }

    /// `Play_SetCameraAtEye`.
    pub fn camera_set_at_eye(&mut self, id: i16, at: Vec3, eye: Vec3) {
        let player_pos = self.player.and_then(|h| self.actors.actor(h)).map(|a| a.world_pos).unwrap_or(Vec3::ZERO);
        if let Some(c) = self.camera_mut(id) {
            c.set_at_eye(at, eye, player_pos);
        }
    }

    /// `Play_SetCameraFov`.
    pub fn camera_set_fov(&mut self, id: i16, fov: f32) {
        if let Some(c) = self.camera_mut(id) {
            c.set_fov(fov);
        }
    }

    /// `Play_CopyCamera`.
    pub fn copy_camera(&mut self, dest: i16, src: i16) {
        let Some(src) = self.camera(src).cloned() else { return };
        let (d, player_pos) = (self.data.clone(), self.player.and_then(|h| self.actors.actor(h)).map(|a| a.world_pos).unwrap_or(Vec3::ZERO));
        if let Some(c) = self.camera_mut(dest) {
            c.copy_from(&d.camera, &src, player_pos);
        }
    }

    /// `Player_GetEnvironmentalHazard` (`z_player_lib.c`): a hot room (`ROOM_ENV_HOT`), under water
    /// (`underwaterTimer` over 80 with the iron boots, or 300 without: on the floor with them, else
    /// free), or swimming (`PLAYER_STATE1_27`), else none. The first time in a hot room without the
    /// Goron Tunic, or under water in the iron boots without the Zora Tunic, Player not in a
    /// cutscene, its text (0x3040, 0x401D) opens once (`envHazardTextTriggerFlags`).
    pub fn player_get_environmental_hazard(&mut self) -> u8 {
        /// `ROOM_ENV_HOT` (`room.h`).
        const ROOM_ENV_HOT: u8 = 3;
        /// `PLAYER_BOOTS_IRON`, `PLAYER_TUNIC_GORON`, `PLAYER_TUNIC_ZORA` (`player.h`).
        const PLAYER_BOOTS_IRON: u8 = 1;
        const PLAYER_TUNIC_GORON: u8 = 1;
        const PLAYER_TUNIC_ZORA: u8 = 2;
        /// `sEnvHazardTextTriggers`, by hazard less one: `ENV_HAZARD_TEXT_TRIGGER_HOTROOM` (1 << 0)
        /// and `_UNDERWATER` (1 << 1), and their texts.
        const TEXT_TRIGGERS: [(u8, u16); 4] = [(1 << 0, 0x3040), (1 << 1, 0x401D), (0, 0x0000), (1 << 1, 0x401D)];
        let Some(p) = self.player.and_then(|h| self.actors.get(h)).and_then(|p| p.as_player()) else { return PLAYER_ENV_HAZARD_NONE };
        let (underwater_timer, boots, tunic, grounded) = p.env_hazard_state();
        let state1 = p.state_flags1();
        let env_hazard = if self.room_ctx.cur.behavior_type2 == ROOM_ENV_HOT {
            PLAYER_ENV_HAZARD_HOTROOM - 1
        } else if underwater_timer > 80 && (boots == PLAYER_BOOTS_IRON || underwater_timer >= 300) {
            if boots == PLAYER_BOOTS_IRON && grounded { PLAYER_ENV_HAZARD_UNDERWATER_FLOOR - 1 } else { PLAYER_ENV_HAZARD_UNDERWATER_FREE - 1 }
        } else if state1 & PLAYER_STATE1_27 != 0 {
            PLAYER_ENV_HAZARD_SWIMMING - 1
        } else {
            return PLAYER_ENV_HAZARD_NONE;
        };
        let (flag, text_id) = TEXT_TRIGGERS[env_hazard as usize];
        if !self.player_in_cs_mode()
            && flag != 0
            && self.save.env_hazard_text_trigger_flags & flag == 0
            && ((env_hazard == PLAYER_ENV_HAZARD_HOTROOM - 1 && tunic != PLAYER_TUNIC_GORON)
                || ((env_hazard == PLAYER_ENV_HAZARD_UNDERWATER_FLOOR - 1 || env_hazard == PLAYER_ENV_HAZARD_UNDERWATER_FREE - 1) && boots == PLAYER_BOOTS_IRON && tunic != PLAYER_TUNIC_ZORA))
        {
            self.start_textbox(text_id, None);
            self.save.env_hazard_text_trigger_flags |= flag;
        }
        env_hazard + 1
    }

    /// `Interface_Update` with this frame's view of play.
    fn interface_update(&mut self) {
        let (state1, state2) = self.player.and_then(|h| self.actors.get(h)).and_then(|p| p.as_player()).map(|pi| (pi.state_flags1(), pi.state_flags2())).unwrap_or((0, 0));
        let f = crate::interface::IfaceFrame {
            scene_id: self.scene_id,
            msg_none: self.msg_ctx.msg_mode == crate::message::MSGMODE_NONE,
            climbing: state1 & PLAYER_STATE1_21 != 0,
            state2_18: state2 & (1 << 18) != 0,
            no_transition: self.transition.trigger == crate::transition::TRANS_TRIGGER_OFF && self.transition.mode == TRANS_MODE_OFF,
            // ROOM_TYPE_DUNGEON.
            dungeon_room: self.room_ctx.cur.behavior_type1 == 1,
            in_cs_mode: self.play_in_cs_mode(),
            paused: self.pause_ctx.is_paused(),
            game_over_inactive: self.game_over_ctx.state == crate::game_over::GAMEOVER_INACTIVE,
            env_hazard: PLAYER_ENV_HAZARD_NONE,
        };
        // Player_GetEnvironmentalHazard, which Interface_Update calls (func_80083108 first, under
        // its message check, then for sEnvHazard every frame): its text opens on its first call,
        // after func_80083108 has read the message box.
        let f = crate::interface::IfaceFrame { env_hazard: if self.interface_ctx.initialised { self.player_get_environmental_hazard() } else { PLAYER_ENV_HAZARD_NONE }, ..f };
        self.interface_ctx.update(&mut self.save, &mut self.audio, &f);
        // Map_Update, which Interface_Update calls between the HUD's fade and the health
        // accumulator (neither reads the other's state).
        if self.interface_ctx.initialised {
            self.map_update();
        }
    }

    /// Runs `f` on the message context with this frame's view of play (nothing without the
    /// messages).
    pub fn with_msg(&mut self, f: impl FnOnce(&mut MessageContext, &mut MsgFrame)) {
        let Some(table) = self.messages.clone() else { return };
        let screen_y = |h: Option<ActorHandle>| h.and_then(|h| self.actors.actor(h)).map(|a| crate::target::actor_screen_pos(self.view_proj, a).1);
        let player_screen_y = screen_y(self.player).unwrap_or(0);
        let talk_actor_screen_y = screen_y(self.msg_ctx.talk_actor);
        let input = self.input;
        let mut frame = MsgFrame {
            table: &table,
            input: &input,
            audio: &mut self.audio,
            save: &mut self.save,
            iface: &mut self.interface_ctx,
            scene_cam_type: self.scene_cam_type,
            scene_id: self.scene_id,
            player_screen_y,
            talk_actor_screen_y,
            cs_idle: self.cs_ctx.state == crate::cutscene::CS_STATE_IDLE,
            active_cam_main: self.active_cam_id == CAM_ID_MAIN,
        };
        f(&mut self.msg_ctx, &mut frame);
    }

    /// `Message_ShouldAdvance`: A, B or C-Up this frame, with the message box's sound.
    pub fn message_should_advance(&mut self) -> bool {
        crate::message::should_advance(&self.input, &mut self.audio)
    }

    /// `Message_StartTextbox`.
    pub fn start_textbox(&mut self, text_id: u16, actor: Option<ActorHandle>) {
        self.with_msg(|m, f| m.start_textbox(f, text_id, actor));
    }

    /// `Message_ContinueTextbox`.
    pub fn continue_textbox(&mut self, text_id: u16) {
        self.with_msg(|m, f| m.continue_textbox(f, text_id));
    }

    /// `Message_GetState(&play->msgCtx)`.
    pub fn message_state(&self) -> u8 {
        self.msg_ctx.get_state()
    }

    /// `Player_InCsMode`.
    pub fn player_in_cs_mode(&self) -> bool {
        let p = self.player.and_then(|h| self.actors.get(h)).and_then(|p| p.as_player());
        self.transition.trigger == crate::transition::TRANS_TRIGGER_START || p.is_some_and(|p| p.in_cs_mode())
    }

    /// `Play_RequestViewpointBgCam`: the viewpoint's bg camera for the active camera.
    pub fn change_viewpoint_bg_cam_index(&mut self) {
        let idx = self.viewpoint as i32 - 1;
        self.game_camera.change_bg_cam_index(&self.data.camera, &self.col, idx);
    }

    /// `Play_SetViewpoint`, with the toggle's sound (not in a shop, nor in a cutscene entered
    /// as one).
    pub fn set_viewpoint(&mut self, viewpoint: u8) {
        use crate::audio::sfx::{NA_SE_SY_CAMERA_ZOOM_DOWN, NA_SE_SY_CAMERA_ZOOM_UP};
        assert!(viewpoint == VIEWPOINT_LOCKED || viewpoint == VIEWPOINT_PIVOT, "point == 1 || point == 2");
        self.viewpoint = viewpoint;
        if self.scene_cam_type != crate::scene::SCENE_CAM_TYPE_FIXED_SHOP_VIEWPOINT && self.save.cutscene_index < 0xFFF0 {
            self.audio.play_sfx_centered(if viewpoint == VIEWPOINT_LOCKED { NA_SE_SY_CAMERA_ZOOM_DOWN } else { NA_SE_SY_CAMERA_ZOOM_UP });
        }
        self.change_viewpoint_bg_cam_index();
    }

    /// `Play_Update`'s viewpoint part: C-Up toggles a house's fixed and pivot cameras (not in a
    /// shop, where it's an error sound, nor in a cutscene), and every frame the viewpoint's bg
    /// camera is asked for.
    fn update_viewpoint(&mut self) {
        if self.viewpoint == VIEWPOINT_NONE {
            return;
        }
        if self.input.press.held(eng_input::pad::BTN_CUP) {
            if self.player_in_cs_mode() {
                // "Changing viewpoint is prohibited during the cutscene".
            } else if self.scene_cam_type == crate::scene::SCENE_CAM_TYPE_FIXED_SHOP_VIEWPOINT {
                self.audio.play_sfx_centered(crate::audio::sfx::NA_SE_SY_ERROR);
            } else {
                self.set_viewpoint(self.viewpoint ^ (VIEWPOINT_LOCKED ^ VIEWPOINT_PIVOT));
            }
        }
        self.change_viewpoint_bg_cam_index();
    }

    /// `Play_CamIsNotFixed`: not a prerendered room, not a fixed-camera scene (the toggle, the
    /// fixed and the market kinds; the shop kind is covered by its rooms), and not the castle
    /// courtyard.
    /// The loaded scene's title file (`play->loadedScene->titleFile`), empty for none.
    pub fn title_file(&self) -> String {
        self.assets.as_ref().and_then(|a| a.scenes.scenes.get(self.scene_id as usize)).map(|s| s.title_file.clone()).unwrap_or_default()
    }

    pub fn cam_is_not_fixed(&self) -> bool {
        use crate::scene::*;
        /// `SCENE_CASTLE_COURTYARD_GUARDS_DAY`.
        const SCENE_CASTLE_COURTYARD_GUARDS_DAY: u16 = 0x45;
        !self.cam_room().image
            && self.scene_cam_type != SCENE_CAM_TYPE_FIXED_TOGGLE_VIEWPOINT
            && self.scene_cam_type != SCENE_CAM_TYPE_FIXED
            && self.scene_cam_type != SCENE_CAM_TYPE_FIXED_MARKET
            && self.scene_id != SCENE_CASTLE_COURTYARD_GUARDS_DAY
    }

    /// What `func_80057FC4` reads of the current room.
    pub fn cam_room(&self) -> crate::camera::CamRoom {
        let room = self.scene.as_ref().and_then(|s| s.room(self.room_ctx.cur.num));
        crate::camera::CamRoom { image: room.is_some_and(|r| r.shape == Some(crate::scene::ShapeKind::Image)), behavior_type1: self.room_ctx.cur.behavior_type1 }
    }

    /// `Actor_UpdateAll`.
    fn update_all_actors(&mut self) {
        self.spawn_setup_actors();
        if self.actors.unk_02 != 0 {
            self.actors.unk_02 -= 1;
        }
        // The actors Player's state doesn't freeze: the one it talks to (sp74, unless its text
        // is Navi's, 0x6xx), Navi, what it holds (nothing here) and its children.
        let pi = self.player.and_then(|h| self.actors.get(h)).and_then(|p| p.as_player());
        let player_state1 = pi.map(|p| p.state_flags1()).unwrap_or(0);
        let talk_actor = pi.and_then(|p| p.talk_target().0);
        let navi = pi.and_then(|p| p.navi_actor());
        let player_text_id = self.player.and_then(|h| self.actors.actor(h)).map(|a| a.text_id).unwrap_or(0);
        let sp74 = (player_state1 & crate::actor_ctx::PLAYER_STATE1_TALKING != 0 && player_text_id & 0xFF00 != 0x600).then_some(talk_actor).flatten();
        let mut exempt = vec![sp74, navi];
        for cat in 0..ACTORCAT_MAX {
            let can_freeze_category = player_state1 & crate::actor_ctx::S_CATEGORY_FREEZE_MASKS[cat] != 0;
            // A snapshot of the list: actors spawned now go to the head and wait for the
            // next frame, as with the decomp's linked lists.
            let mut list = self.actors.category(cat).to_vec();
            let mut i = 0;
            while i < list.len() {
                let h = list[i];
                i += 1;
                // Player updates before the later categories, so they see its new position.
                let player_pos = self.player.and_then(|p| self.actors.actor(p)).map(|a| a.world_pos);
                // actor->parent == &player->actor.
                let child_of_player = self.player.is_some() && self.actors.actor(h).is_some_and(|a| a.parent == self.player);
                exempt.truncate(2);
                if child_of_player {
                    exempt.push(Some(h));
                }
                self.update_actor(h, player_pos, can_freeze_category, &exempt);
                // An actor that changed its category in its update (Actor_ChangeCategory: to the
                // head of the other list) leaves the loop's `actor = actor->next` in that list:
                // the rest of this category waits for the next frame, and the other list's
                // actors after it update here (with this category's freeze mask), and again in
                // their own category's turn if that's later.
                if let Some(c) = self.actors.actor(h).map(|a| a.category).filter(|&c| c != cat)
                    && let Some(at) = self.actors.category(c).iter().position(|&x| x == h)
                {
                    list = self.actors.category(c)[at + 1..].to_vec();
                    i = 0;
                }
            }
            if cat == ACTORCAT_BG {
                self.col.dyna.update_context();
            }
        }
        // Attention_Update with the actor Player keeps targeted, against the last frame's view.
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
                view_eye: self.view_eye(),
            };
            crate::target::update(&mut self.target_ctx, &self.actors, &frame, &mut self.audio);
        }
        // TitleCard_Update, then DynaPoly_UpdateBgActorTransforms.
        self.title_ctx.update();
        self.col.dyna.update_prev_transforms();
    }

    /// `play->view.eye` for the active camera.
    fn view_eye(&self) -> Vec3 {
        match self.camera_kind {
            CameraKind::Game => self.active_camera().shaken_view().eye,
            CameraKind::Follow => self.follow_camera.eye(),
        }
    }

    /// `play->viewProjectionMtxF` for the active camera: the game's 320x240 view, `zNear` 10.
    fn camera_view_proj(&self) -> Mat4 {
        let (eye, at, fov) = match self.camera_kind {
            CameraKind::Game => {
                let v = self.active_camera().shaken_view();
                (v.eye, v.at, v.fov)
            }
            CameraKind::Follow => (self.follow_camera.eye(), self.follow_camera.at, 50.0),
        };
        eng_math::gu_perspective(fov, 4.0 / 3.0, 10.0, 12800.0) * glam::camera::rh::view::look_at_mat4(eye, at, Vec3::Y)
    }

    /// The render state of every actor and the camera now.
    fn capture(&mut self) -> RenderFrame {
        let mut actors = Vec::new();
        for h in self.actors.all() {
            if let Some(a) = self.actors.get_mut(h) {
                let mut rs = a.render_state();
                rs.color_filter = (a.base().color_filter_params, a.base().color_filter_timer);
                actors.push((h, rs));
                a.base_mut().teleported = false;
            }
        }
        let view = match self.camera_kind {
            CameraKind::Game => self.active_camera().shaken_view(),
            CameraKind::Follow => CamView { eye: self.follow_camera.eye(), at: self.follow_camera.at, fov: 50.0 },
        };
        let active = self.active_cam_id;
        let mut view_cut = self.captured_cam_id != active;
        self.captured_cam_id = active;
        for id in CAM_ID_MAIN..NUM_CAMS as i16 {
            if let Some(c) = self.camera_mut(id) {
                view_cut |= c.view_cut && id == active;
                c.view_cut = false;
            }
        }
        let view_cut = view_cut && self.camera_kind == CameraKind::Game;
        RenderFrame { actors, view, follow: self.follow_camera, letterbox: self.letterbox.rows() as f32, view_cut }
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
        self.advance_with(dt, |_| {})
    }

    /// `advance`, with `after` run after each frame: the audio side's hand-over
    /// (`GameAudio::take_ops`, `GameAudio::set_view`).
    pub fn advance_with(&mut self, dt: f32, mut after: impl FnMut(&mut crate::audio::GameAudio)) -> u32 {
        self.acc += dt.min(0.25);
        let step = 1.0 / GAME_HZ;
        let mut n = 0;
        while self.acc >= step && n < 5 {
            self.acc -= step;
            self.tick();
            after(&mut self.audio);
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
            _ => RenderFrame { actors: Vec::new(), view: CamView { eye: Vec3::ZERO, at: Vec3::Z, fov: 60.0 }, follow: self.follow_camera, letterbox: 0.0, view_cut: false },
        }
    }

    /// The last game frame's render state, unblended.
    pub fn current_frame(&self) -> RenderFrame {
        self.cur.clone().unwrap_or_else(|| self.render_frame())
    }

    /// `Actor_DrawAll` (every actor from its render state `frame`), then the target reticle.
    /// `Actor_Draw` binds the light list's point lights at the actor's position
    /// (`Lights_BindAll`; none with `ACTOR_FLAG_IGNORE_POINT_LIGHTS`) for everything the actor draws.
    pub fn draw(&self, frame: &RenderFrame, view: &ViewInfo, out: &mut DrawOut) {
        // Play_Draw's weather before Actor_DrawAll: the bolts (before the rooms in the C) and
        // the rain.
        crate::weather::draw(&self.rain, &self.lightning_bolts, view, out);
        for (h, rs) in &frame.actors {
            if let Some(a) = self.actors.get(*h)
                && !a.base().killed
            {
                let (opa, xlu) = (out.opa.len(), out.xlu.len());
                a.draw(rs, self, view, out);
                // Actor_Draw's colour filter: the fog its draws in one list take, unless they set
                // their own.
                if let Some((fog, xlu_list)) = color_filter_fog(rs.color_filter) {
                    let list = if xlu_list { &mut out.xlu[xlu..] } else { &mut out.opa[opa..] };
                    for c in list.iter_mut().filter(|c| c.params.fog.is_none()) {
                        c.params.fog = Some(fog);
                    }
                }
                let at = (a.base().flags & crate::actor::ACTOR_FLAG_IGNORE_POINT_LIGHTS == 0).then_some(rs.pos);
                let lights: Vec<eng_gfx::PointLight> = self.light_ctx.bind_all(at).into_iter().map(|l| eng_gfx::PointLight { dir: l.dir, color: l.color }).collect();
                if !lights.is_empty() {
                    for c in out.opa[opa..].iter_mut().chain(out.xlu[xlu..].iter_mut()) {
                        c.params.lights = lights.clone();
                    }
                }
            }
        }
        let _ = view;
        // The end of Actor_DrawAll: the effects (drawn in the game frame, crate::effect), then
        // TitleCard_Draw.
        out.opa.extend(self.effect_draws.opa.iter().cloned());
        out.xlu.extend(self.effect_draws.xlu.iter().cloned());
        let mut title = Vec::new();
        self.title_ctx.draw(&mut title);
        out.overlay_2d.extend(title.iter().map(|s| s.draw_cmd()));
        // Play_DrawOverlayElements → Interface_Draw: the HUD either side of the reticle, then
        // the message box.
        let hud = |f: fn(&InterfaceContext, &SaveContext, &mut Vec<crate::sprite::Sprite>), out: &mut DrawOut| {
            if self.interface_ctx.initialised {
                let mut sprites = Vec::new();
                f(&self.interface_ctx, &self.save, &mut sprites);
                out.overlay_2d.extend(sprites.iter().map(|s| s.draw_cmd()));
            }
        };
        hud(InterfaceContext::draw_hud_1, out);
        crate::target::draw(&self.target_ctx, &self.actors, self.gameplay_frames, out);
        hud(InterfaceContext::draw_hud_2, out);
        self.msg_ctx.draw(out);
        out.letterbox_rows = frame.letterbox;
        // The fills, where Play_Draw draws them (the transition's under the overlay's HUD and
        // message box).
        let (env, fade) = self.draw_fills();
        out.opa_fill = env;
        out.xlu_fill = env;
        // Interface_Draw's black (unk_244, the game over's fade) over the transition's fade.
        // (The C draws it after the HUD, which the game over has hidden by then.)
        let unk_244 = self.interface_ctx.unk_244;
        out.overlay_fill = if unk_244 > 0 { Some(match fade { Some(f) => crate::play_scene::compose_fill(f, [0, 0, 0, unk_244 as u8]), None => [0, 0, 0, unk_244 as u8] }) } else { fade };
    }
}

/// `Actor_Draw`'s colour filter (`colorFilterParams`, `colorFilterTimer`): the colour (grey,
/// red or blue at the filter's intensity), then `func_80026400` (OPA) or `func_80026860` (XLU):
/// `gSPFogPosition(0, 2800 * |cos| + 1700)` over the duration. Returns the fog and whether
/// it's the XLU list's.
pub fn color_filter_fog((params, timer): (u16, u8)) -> Option<(eng_gfx::FogOverride, bool)> {
    use crate::actor::*;
    if timer == 0 {
        return None;
    }
    let intensity = colorfilter_get_colorintensity(params) | 7;
    let mut color = [0u8, 0, 0, 255];
    if params & COLORFILTER_COLORFLAG_GRAY != 0 {
        color = [intensity, intensity, intensity, 255];
    } else if params & COLORFILTER_COLORFLAG_RED != 0 {
        color[0] = intensity;
    } else {
        color[2] = intensity;
    }
    let duration = colorfilter_get_duration(params) as i16;
    // (PLATFORM_GC: a duration of 0 sets no fog.)
    if duration == 0 {
        return None;
    }
    let cos = eng_math::cos_s(((0x4000 / duration) as i32 * timer as i32) as i16);
    let far = (2800.0 * cos.abs()) as i16 as i32 + 1700;
    Some((crate::gbi::sp_fog_position(color, 0, far), params & COLORFILTER_BUFFLAG_XLU != 0))
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

/// `Actor_Draw`'s full model matrix from a render state: `Matrix_SetTranslateRotateYXZ` at the
/// position raised by `shape.yOffset * scale.y`, with the shape rotation (Y, then X, then Z),
/// then `Matrix_Scale`.
pub fn actor_draw_matrix(rs: &RenderState) -> Mat4 {
    let r = |a: i16| eng_math::binang_to_rad(a);
    Mat4::from_translation(rs.pos + Vec3::Y * (rs.y_offset * rs.scale.y))
        * Mat4::from_rotation_y(r(rs.rot[1]))
        * Mat4::from_rotation_x(r(rs.rot[0]))
        * Mat4::from_rotation_z(r(rs.rot[2]))
        * Mat4::from_scale(rs.scale)
}

/// The game's random number generator (`qrand.c`, and `z_actor.c`'s float helpers).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rand {
    /// `sRandInt`.
    pub state: u32,
}

impl Default for Rand {
    /// `static u32 sRandInt = 1`.
    fn default() -> Rand {
        Rand { state: 1 }
    }
}

impl Rand {
    /// `Rand_Next`.
    pub fn next(&mut self) -> u32 {
        self.state = self.state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.state
    }
    /// `Rand_ZeroOne`: [0, 1).
    pub fn zero_one(&mut self) -> f32 {
        let s = self.next();
        f32::from_bits((s >> 9) | 0x3F80_0000) - 1.0
    }
    /// `Rand_Centered`: [-0.5, 0.5).
    pub fn centered(&mut self) -> f32 {
        let s = self.next();
        f32::from_bits((s >> 9) | 0x3F80_0000) - 1.5
    }
    /// `Rand_ZeroFloat`.
    pub fn zero_float(&mut self, f: f32) -> f32 {
        self.zero_one() * f
    }
    /// `Rand_CenteredFloat`.
    pub fn centered_float(&mut self, f: f32) -> f32 {
        (self.zero_one() - 0.5) * f
    }
    /// `Rand_S16Offset`.
    pub fn s16_offset(&mut self, base: i16, range: i16) -> i16 {
        ((self.zero_one() * range as f32) as i16).wrapping_add(base)
    }
}

/// `!Object_IsLoaded(&play->objectCtx, actor->objectSlot)` for an initialised actor that
/// spawned by id (actors built directly have no bank).
fn base_bank_dropped(a: &Actor, objects: &ObjectContext) -> bool {
    a.obj_bank_index.is_some_and(|b| !objects.is_loaded(b))
}
