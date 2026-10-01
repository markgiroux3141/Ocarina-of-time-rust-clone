//! Entering scenes and changing rooms: `Play_Init`, `Play_SpawnScene` and the scene and room
//! header commands (`z_play.c`, `z_scene.c`), the room loads (`z_room.c`), exits and
//! respawns, and the transition state machine at the top of `Play_Update`.
//!
//! ## Entering a scene
//!
//! `PlayState::play_init` does what `Play_Init` does:
//! - `nightFlag` from the day time; the scene layer: a cutscene layer for a `cutsceneIndex` of
//!   0xFFF0 and up (`SCENE_LAYER_CUTSCENE_FIRST + (cutsceneIndex & 0xF)`), else from Link's age
//!   and the time (and the Hyrule Field / Kokiri Forest special cases, with no quest items or
//!   event flags set);
//! - `Play_SpawnScene` with the entrance table's scene and spawn for that layer: the scene's
//!   header (collision, the entrance, exit and transition-actor lists, the keep object, Link's
//!   object, the light settings) and the first room's load (`func_80096FE8`: the entrance's
//!   room, or the respawn point's);
//! - `func_800304DC`: the actor context, and Player spawned from the spawn list's entry;
//! - the first room's load finished (`func_800973FC`): its header (actor list, object list,
//!   behaviour) and the transition actors;
//! - the cameras on Player, `AnimationContext_Update`, `respawnFlag` cleared;
//! - `transitionTrigger = TRANS_TRIGGER_END`, so the first frame starts the fade in.
//!
//! ## Leaving
//!
//! Player sets `nextEntranceIndex` and `transitionTrigger = TRANS_TRIGGER_START` on an exit
//! (or `Play_TriggerVoidOut`). The transition runs while play goes on, and when it covers the
//! screen `Play_Update` ends the game state: `gSaveContext.entranceIndex` becomes the next
//! entrance and a new `Play_Init` runs. Here that's `reinit` at the end of the frame: a new
//! play state from the same save, pad and assets.

use std::sync::Arc;

use anyhow::{Context, Result};
use eng_collision::bgcheck::CollisionContext;
use eng_gfx::SegmentValues;
use glam::Vec3;

use crate::actor_table::ActorTable;
use crate::env::{self, EnvLights, EnvState, EnvTables};
use crate::object_ctx::{OBJECT_LINK_BOY, OBJECT_LINK_CHILD, ObjectContext};
use crate::pack::GamePack;
use crate::play::PlayState;
use crate::room::{Room, RoomContext};
use crate::save::{RESPAWN_MODE_DOWN, SaveContext, TRANS_NEXT_TYPE_DEFAULT};
use crate::scene::{ActorEntry, LayerData, RoomData, SceneData, SceneTable, layer_for};
use crate::scene_table::{self, DrawConfigState};
use crate::spawn::Overlays;
use crate::transition::*;

/// `SCENE_*` ids with special cases in `Play_Init` (`include/tables/scene_table.h`).
pub const SCENE_SPOT00: u16 = 0x51;
pub const SCENE_SPOT04: u16 = 0x55;
/// `SCENE_YOUSEI_IZUMI_TATE`, `SCENE_KAKUSIANA`: `Play_SetupRespawnPoint` does nothing there.
pub const SCENE_YOUSEI_IZUMI_TATE: u16 = 0x3C;
pub const SCENE_KAKUSIANA: u16 = 0x3E;
/// `SCENE_HAKADAN`, `SCENE_GANON_FINAL`: special cases of Player's void check.
pub const SCENE_HAKADAN: u16 = 0x07;
pub const SCENE_GANON_FINAL: u16 = 0x1A;
/// `R_UPDATE_RATE`.
pub const R_UPDATE_RATE: u16 = 3;

/// What play needs from the asset pack and the content crate, shared by every play state of a
/// session (a scene change keeps them).
pub struct GameAssets {
    pub pack: GamePack,
    pub scenes: SceneTable,
    pub actors: ActorTable,
    pub env: EnvTables,
    /// The ported actors' constructors.
    pub overlays: Overlays,
    /// The English messages.
    pub messages: Arc<crate::message::MessageTable>,
    /// The random drops' tables.
    pub item_drops: crate::item::ItemDropTables,
    /// `sRestrictionFlags`.
    pub interface: crate::interface::InterfaceTables,
    /// `sGetItemTable`, `sDrawItemTable`.
    pub items: crate::item::ItemTables,
    /// `sEntranceCutsceneTable` and the scripts' keys (docs/adr/0022-cutscenes.md).
    pub cutscenes: crate::cutscene::CutsceneTables,
    /// Navi's C-Up texts and Saria's (`z_elf_message.c`).
    pub elf_messages: crate::elf_message::ElfMessageTables,
    /// Cutscene scripts read so far, by symbol.
    scripts: std::sync::Mutex<std::collections::HashMap<String, Arc<crate::cutscene::CutsceneScript>>>,
    /// Skeletons and standard animations read so far (actors load theirs at init).
    skeletons: std::sync::Mutex<std::collections::HashMap<String, Arc<eng_anim::skeleton::Skeleton>>>,
    animations: std::sync::Mutex<std::collections::HashMap<String, Arc<eng_anim::anim::StandardAnimation>>>,
}

impl GameAssets {
    pub fn load(pack: GamePack, overlays: Overlays) -> Result<GameAssets> {
        Ok(GameAssets {
            scenes: pack.scene_table()?,
            actors: pack.actor_table()?,
            env: pack.env_tables()?,
            messages: Arc::new(pack.messages()?),
            item_drops: pack.item_drops()?,
            interface: pack.interface()?,
            items: pack.items()?,
            cutscenes: pack.cutscene_tables()?,
            elf_messages: pack.elf_messages()?,
            scripts: Default::default(),
            overlays,
            pack,
            skeletons: Default::default(),
            animations: Default::default(),
        })
    }

    /// The cutscene script named `name` (`D_808BCE20`, `gDekuTreeIntroCs`), read once.
    pub fn cutscene(&self, name: &str) -> Result<Arc<crate::cutscene::CutsceneScript>> {
        if let Some(s) = self.scripts.lock().unwrap().get(name) {
            return Ok(s.clone());
        }
        let key = self.cutscenes.key(name).with_context(|| format!("no cutscene script {name}"))?;
        let s = Arc::new(self.pack.cutscene(key)?);
        self.scripts.lock().unwrap().insert(name.to_string(), s.clone());
        Ok(s)
    }

    /// The script under pack key `key` (a scene layer's `cutscene`).
    pub fn cutscene_by_key(&self, key: &str) -> Result<Arc<crate::cutscene::CutsceneScript>> {
        let name = self.cutscenes.scripts.iter().find(|(_, k)| k == key).map(|(n, _)| n.clone()).with_context(|| format!("no cutscene script {key}"))?;
        self.cutscene(&name)
    }

    /// `file`'s skeleton `symbol`, read once.
    pub fn skeleton(&self, file: &str, symbol: &str) -> Result<Arc<eng_anim::skeleton::Skeleton>> {
        let key = crate::pack::keys::skeleton(file, symbol);
        if let Some(s) = self.skeletons.lock().unwrap().get(&key) {
            return Ok(s.clone());
        }
        let s = Arc::new(self.pack.skeleton(file, symbol)?);
        self.skeletons.lock().unwrap().insert(key, s.clone());
        Ok(s)
    }

    /// `file`'s standard animation `symbol`, read once.
    pub fn animation(&self, file: &str, symbol: &str) -> Result<crate::skelanime_std::Anim> {
        let key = crate::pack::keys::anim(file, symbol);
        let cached = self.animations.lock().unwrap().get(&key).cloned();
        let data = match cached {
            Some(a) => a,
            None => {
                let a = Arc::new(self.pack.standard_animation(file, symbol)?);
                self.animations.lock().unwrap().insert(key, a.clone());
                a
            }
        };
        Ok(crate::skelanime_std::Anim { name: symbol.to_string(), data })
    }
}

/// A scene as play has it loaded: the header for its layer, its rooms, the environment's
/// lights, and the scene draw config's state.
#[derive(Clone)]
pub struct SceneState {
    pub data: SceneData,
    /// `gSaveContext.sceneLayer`.
    pub layer: usize,
    /// Every room of the layer (the room context says which are loaded).
    pub rooms: Vec<Arc<RoomData>>,
    pub lights: EnvLights,
    /// The draw config's inputs and its state across frames, and this frame's segment values
    /// (OPA, XLU), or `None` if the draw config isn't ported.
    pub draw: DrawConfigState,
    pub segments: Option<[SegmentValues; 2]>,
    /// Draw every room (the spikes' view of a scene) instead of the room context's.
    pub all_rooms: bool,
}

impl SceneState {
    /// Loads `name`'s layer `layer` at `day_time`: its rooms, and the lights
    /// `Environment_Init` + `Environment_Update` give at that time (room 0's time settings
    /// apply, as `Scene_CommandTimeSettings` runs from the room header).
    pub fn load(pack: &GamePack, env_tables: &EnvTables, name: &str, layer: usize, child: bool, night: bool, day_time: u16) -> Result<SceneState> {
        let data = pack.scene(name)?;
        let (stored, ld) = data.layer(layer);
        let rooms = ld.rooms.iter().map(|k| pack.room(k).map(Arc::new)).collect::<Result<Vec<_>>>()?;
        let (day, sky, _speed) = env::scene_times(day_time, rooms.first().and_then(|r| r.time));
        let lights = env::update(
            env_tables,
            &ld.light_settings,
            &EnvState { light_mode: ld.skybox.light_mode, light_config: 0, light_setting: 0, day_time: day, skybox_time: sky },
        );
        let draw = DrawConfigState { gameplay_frames: 0, child, night, scene_layer: layer, day_time: day, ..Default::default() };
        Ok(SceneState { data, layer: stored, rooms, lights, draw, segments: None, all_rooms: false })
    }

    pub fn layer_data(&self) -> &LayerData {
        &self.data.layers[self.layer]
    }

    pub fn room(&self, num: i8) -> Option<&Arc<RoomData>> {
        usize::try_from(num).ok().and_then(|i| self.rooms.get(i))
    }

    /// The scene's name without `_scene`.
    pub fn short_name(&self) -> &str {
        self.data.name.strip_suffix("_scene").unwrap_or(&self.data.name)
    }

    /// Runs the scene draw config for this game frame (`Scene_Draw`).
    pub fn run_draw_config(&mut self, gameplay_frames: u32) {
        self.draw.gameplay_frames = gameplay_frames;
        self.segments = scene_table::segment_values(&self.data.draw_config, &mut self.draw);
    }
}

/// The transition state `Play_Update` steps through.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TransitionState {
    /// `transitionTrigger`, `transitionMode`, `transitionType`.
    pub trigger: i8,
    pub mode: u8,
    pub ty: u8,
    /// The type `Play_SetupTransition` got (`transitionCtx.transitionType`).
    pub ctx_type: u8,
    /// `nextEntranceIndex`.
    pub next_entrance_index: u16,
    /// `transitionCtx`'s instance (every instance type is a fade here).
    pub fade: TransitionFade,
    /// `sTransitionFillTimer`, and `envCtx.fillScreen` / `screenFillColor`.
    pub fill_timer: u16,
    pub screen_fill: Option<[u8; 4]>,
}

/// The part of `PlayState` an actor's update may write besides the actors (what the decomp's
/// actors reach through `play->` and `gSaveContext`): the transition and exit fields, the
/// save, and the scene flags. `PlayState::take_io` lends it out for Player's update, and
/// `put_io` takes it back.
#[derive(Debug, Clone, Default)]
pub struct PlayIo {
    pub save: SaveContext,
    pub transition: TransitionState,
    pub flags: crate::spawn::SceneFlags,
    /// Read-only here: `sceneId`, `roomCtx.curRoom.num`, `curSpawn`.
    pub scene_id: u16,
    pub room: i8,
    pub cur_spawn: usize,
}

impl PlayIo {
    /// `Play_SetRespawnData`.
    pub fn set_respawn_data(&mut self, mode: usize, entrance_index: u16, room: u8, params: i16, pos: Vec3, yaw: i16) {
        let r = &mut self.save.respawn[mode];
        r.entrance_index = entrance_index;
        r.room_index = room;
        r.pos = pos;
        r.yaw = yaw;
        r.player_params = params;
        r.temp_swch_flags = self.flags.temp_swch;
        r.temp_collect_flags = self.flags.temp_collect;
    }

    /// `Play_SetupRespawnPoint` with Player at `pos` facing `yaw`.
    pub fn setup_respawn_point(&mut self, mode: usize, params: i16, pos: Vec3, yaw: i16) {
        if self.scene_id != SCENE_YOUSEI_IZUMI_TATE && self.scene_id != SCENE_KAKUSIANA {
            let (entr, room) = (self.save.entrance_index, self.room as u8);
            self.set_respawn_data(mode, entr, room, params, pos, yaw);
        }
    }

    /// `Play_TriggerVoidOut`.
    pub fn trigger_void_out(&mut self) {
        let r = &mut self.save.respawn[RESPAWN_MODE_DOWN];
        r.temp_swch_flags = self.flags.temp_swch;
        r.temp_collect_flags = self.flags.temp_collect;
        self.save.respawn_flag = 1;
        self.transition.trigger = TRANS_TRIGGER_START;
        self.transition.next_entrance_index = self.save.respawn[RESPAWN_MODE_DOWN].entrance_index;
        self.transition.ty = TRANS_TYPE_FADE_BLACK;
    }

    /// `Play_LoadToLastEntrance` (without the Ganon's Castle and Hyrule Field special cases).
    pub fn load_to_last_entrance(&mut self) {
        self.save.respawn_flag = -1;
        self.transition.trigger = TRANS_TRIGGER_START;
        self.transition.next_entrance_index = self.save.entrance_index;
        self.transition.ty = TRANS_TYPE_FADE_BLACK;
    }

    /// `Play_TriggerRespawn` with Player at `pos` facing `yaw`.
    pub fn trigger_respawn(&mut self, pos: Vec3, yaw: i16) {
        self.setup_respawn_point(RESPAWN_MODE_DOWN, 0xDFF, pos, yaw);
        self.load_to_last_entrance();
    }

    /// `Scene_SetTransitionForNextEntrance`: the leaving half of the next entrance's layer row.
    pub fn set_transition_for_next_entrance(&mut self, entrances: &[crate::scene::EntranceInfo]) {
        let i = self.transition.next_entrance_index as usize
            + match (self.save.is_day(), self.save.adult) {
                (false, false) => 1,
                (false, true) => 3,
                (true, false) => 0,
                (true, true) => 2,
            };
        if let Some(e) = entrances.get(i) {
            self.transition.ty = e.start_trans_type;
        }
    }
}

impl PlayState {
    /// Lends out the fields an actor's update may write (`PlayIo`).
    pub fn take_io(&mut self) -> PlayIo {
        PlayIo {
            save: std::mem::take(&mut self.save),
            transition: self.transition,
            flags: self.flags,
            scene_id: self.scene_id,
            room: self.room_ctx.cur.num,
            cur_spawn: self.cur_spawn,
        }
    }

    /// Takes back what `take_io` lent.
    pub fn put_io(&mut self, io: PlayIo) {
        self.save = io.save;
        self.transition = io.transition;
        self.flags = io.flags;
    }

    fn with_io<R>(&mut self, f: impl FnOnce(&mut PlayIo) -> R) -> R {
        let mut io = self.take_io();
        let r = f(&mut io);
        self.put_io(io);
        r
    }

    /// `Play_Init` for `save.entrance_index`, with Player and every actor from the pack's
    /// actor table (`assets.overlays` for the ported ones, placeholders for the rest).
    pub fn play_init(assets: Arc<GameAssets>, data: Arc<crate::data::GameData>, rules: Arc<crate::player_lib::PlayerRules>, mut save: SaveContext) -> Result<PlayState> {
        let t = &assets.scenes;
        // (func_8006450C comes with the new play state's idle csCtx.)
        if save.next_cutscene_index != 0xFFEF {
            save.cutscene_index = save.next_cutscene_index;
            save.next_cutscene_index = 0xFFEF;
        }
        if save.cutscene_index == 0xFFFD {
            save.cutscene_index = 0;
        }
        save.night_flag = save.day_time > env::clock_time(18, 0) as u16 || save.day_time < env::clock_time(6, 30) as u16;
        crate::cutscene::handle_conditional_triggers(&assets, &mut save);
        if save.game_mode != crate::save::GAMEMODE_NORMAL || save.cutscene_index >= 0xFFF0 {
            // (nayrusLoveTimer = 0, Magic_Reset: neither is ported.)
            save.scene_layer = crate::scene::SCENE_LAYER_CUTSCENE_FIRST + (save.cutscene_index as usize & 0xF);
        } else {
            save.scene_layer = layer_for(!save.adult, save.night_flag);
        }
        let base_layer = save.scene_layer;
        let base = t.entrances.get(save.entrance_index as usize).with_context(|| format!("no entrance {:#x}", save.entrance_index))?;
        // Play_Init's special cases (no Spiritual Stones, EVENTCHKINF_48 unset), outside the
        // cutscene layers (!IS_CUTSCENE_LAYER).
        let cutscene_layer = save.scene_layer > 3;
        if base.scene == SCENE_SPOT00 && !save.adult && !cutscene_layer {
            save.scene_layer = 0;
        } else if base.scene == SCENE_SPOT04 && save.adult && !cutscene_layer {
            save.scene_layer = 2;
        }
        let entr = t.entrances.get(save.entrance_index as usize + save.scene_layer).with_context(|| format!("entrance {:#x} has no layer {}", save.entrance_index, save.scene_layer))?;
        let (scene_id, spawn) = (entr.scene, entr.spawn as usize);
        let scene_file = t.scenes.get(scene_id as usize).with_context(|| format!("no scene {scene_id}"))?.file.clone();

        // Play_SpawnScene → Play_InitScene → Scene_ExecuteCommands.
        let scene = SceneState::load(&assets.pack, &assets.env, &scene_file, save.scene_layer, !save.adult, save.night_flag, save.day_time)?;
        let ld = scene.layer_data().clone();
        let col = CollisionContext::new(assets.pack.collision(&ld.collision)?);
        let mut play = PlayState::new(data, rules, col, (Vec3::ZERO, 0), save.adult);
        play.assets = Some(assets.clone());
        play.messages = Some(assets.messages.clone());
        play.scene_id = scene_id;
        play.cur_spawn = spawn;
        play.object_ctx = ObjectContext::init_bank();
        play.room_ctx = RoomContext::default();
        // SCENE_CMD_ID_SPECIAL_FILES (the keep object, Navi's C-Up texts), then
        // SCENE_CMD_ID_SPAWN_LIST (Link's object).
        if let Some(k) = ld.keep_object_id {
            play.object_ctx.sub_keep_index = play.object_ctx.spawn(k);
        }
        if ld.c_up_elf_msg_num != 0 {
            play.c_up_elf_msgs = Some(ld.c_up_elf_msg_num as usize - 1);
        }
        let entrance = ld.entrances.get(spawn).copied().with_context(|| format!("{scene_file}: no entrance-list entry {spawn}"))?;
        let link_entry: ActorEntry = *ld.spawns.get(entrance.spawn as usize).with_context(|| format!("{scene_file}: no spawn {}", entrance.spawn))?;
        play.link_object_id = if save.adult { OBJECT_LINK_BOY } else { OBJECT_LINK_CHILD };
        play.object_ctx.spawn(play.link_object_id);
        play.transi_actors = ld.transition_actors.clone();
        // SCENE_CMD_ID_CUTSCENE_DATA: Scene_CommandCutsceneData.
        if let Some(k) = &ld.cutscene {
            match assets.cutscene_by_key(k) {
                Ok(s) => play.cs_ctx.segment = Some(s),
                Err(e) => log::error!("{e:#}"),
            }
        }
        play.scene = Some(scene);

        // func_80096FE8: the first room.
        let front = if save.respawn_flag > 0 { save.respawn[save.respawn_flag as usize - 1].room_index } else { entrance.room };
        play.room_ctx.request(front as i8);

        play.transition.trigger = TRANS_TRIGGER_END;
        play.transition.ty = if save.next_transition_type == TRANS_NEXT_TYPE_DEFAULT {
            t.entrances.get(save.entrance_index as usize + base_layer).map(|e| e.end_trans_type).unwrap_or(TRANS_TYPE_FADE_BLACK)
        } else {
            let ty = save.next_transition_type;
            save.next_transition_type = TRANS_NEXT_TYPE_DEFAULT;
            ty
        };
        play.save = save;
        // Environment_Init (SCENE_CMD_ID_SKYBOX_SETTINGS' Play_InitEnvironment):
        // cutsceneTransitionControl = 0 (z_kankyo.c:318), D_8015FCC8 = 1 (z_kankyo.c:419), and no
        // cues.
        play.save.cutscene_transition_control = 0;
        play.demo.d_8015fcc8 = 1;
        play.cs_ctx.npc_actions = [None; 10];
        // Cutscene_HandleEntranceTriggers, after Play_SpawnScene.
        play.cutscene_handle_entrance_triggers();
        // Interface_Init (Interface_SetSceneRestrictions comes later in Play_Init; nothing reads
        // the restrictions between).
        play.interface_ctx = crate::interface::InterfaceContext::init(&mut play.save, &assets.interface, scene_id);

        // func_800304DC (Actor_InitContext): the scene's saved flags, then Player.
        let saved = play.save.scene_flags(scene_id);
        play.flags = crate::spawn::SceneFlags { chest: saved.chest, swch: saved.swch, clear: saved.clear, collect: saved.collect, ..Default::default() };
        let p = play.actor_spawn_entry(&link_entry).map_err(|e| anyhow::anyhow!("spawning Player: {e:?}"))?;
        play.player = Some(p);
        // func_8002C0C0 (Actor_InitContext's, once Player is in): Navi's point at Player.
        if let Some(a) = play.actors.actor(p).cloned() {
            let eye = play.game_camera.eye;
            play.target_ctx.func_8002c0c0(&a, eye);
        }
        play.spawn = (Vec3::new(link_entry.pos[0] as f32, link_entry.pos[1] as f32, link_entry.pos[2] as f32), link_entry.rot[1]);
        while !play.room_finish_load() {}
        // Camera_InitPlayerSettings (with func_8005AC48's 0xFF from earlier in Play_Init), then
        // Camera_ChangeMode(NORMAL), and Player's start bg camera (params & 0xFF).
        play.reset_cameras();
        // Camera_Init's and Camera_InitPlayerSettings' shared state (sNextUID carries over).
        play.cam_globals = crate::camera::CameraGlobals { next_uid: play.cam_globals.next_uid, ..crate::camera::CameraGlobals::main_init() };
        play.game_camera.change_mode(&play.data.camera, crate::camera::CAM_MODE_NORMAL);
        let start_bg_cam = play.actors.actor(p).map(|a| a.params as u16 & 0xFF).unwrap_or(0xFF);
        if start_bg_cam != 0xFF {
            play.game_camera.change_bg_cam_index(&play.data.camera, &play.col, start_bg_cam as i32);
        }
        play.scene_cam_type = ld.scene_cam_type;
        play.viewpoint = match ld.scene_cam_type {
            crate::scene::SCENE_CAM_TYPE_FIXED_TOGGLE_VIEWPOINT => crate::play::VIEWPOINT_PIVOT,
            crate::scene::SCENE_CAM_TYPE_FIXED_SHOP_VIEWPOINT => crate::play::VIEWPOINT_LOCKED,
            _ => crate::play::VIEWPOINT_NONE,
        };
        for h in play.actors.all() {
            if let Some(a) = play.actors.get_mut(h) {
                a.animation_update();
            }
        }
        play.save.respawn_flag = 0;
        play.reset_blending();
        Ok(play)
    }

    /// Ends this play state and runs `Play_Init` for `save.entrance_index` in its place, keeping
    /// the save, the pad, the camera choice and the debug switches.
    pub(crate) fn reinit(&mut self) {
        let Some(assets) = self.assets.clone() else { return };
        // Play_Destroy → Actor_CleanupContext → Play_SaveSceneFlags.
        self.save_scene_flags();
        match PlayState::play_init(assets, self.data.clone(), self.rules.clone(), self.save.clone()) {
            Ok(mut next) => {
                // Until the new state's first frame runs, the screen keeps the fade-out this
                // one finished on.
                next.pre_update_fill = self.screen_fill();
                next.pad = std::mem::take(&mut self.pad);
                next.debug = self.debug;
                next.scene_changes = self.scene_changes + 1;
                if next.camera_kind != self.camera_kind {
                    next.toggle_camera();
                }
                next.respawn_player = self.respawn_player;
                // z_demo.c's statics and sNextUID are the code segment's: they carry over.
                next.demo = crate::cutscene::DemoStatics { d_8015fcc8: next.demo.d_8015fcc8, ..self.demo };
                next.cam_globals.next_uid = self.cam_globals.next_uid;
                *self = next;
            }
            Err(e) => {
                log::error!("Play_Init for entrance {:#x}: {e:#}", self.save.entrance_index);
                self.transition = TransitionState::default();
            }
        }
    }

    /// `Play_SaveSceneFlags`: the scene's chest, switch, clear and collectible flags into the
    /// save, for the next visit.
    pub fn save_scene_flags(&mut self) {
        let f = self.flags;
        if let Some(s) = self.save.scene_flags.get_mut(self.scene_id as usize) {
            s.chest = f.chest;
            s.swch = f.swch;
            s.clear = f.clear;
            s.collect = f.collect;
        }
    }

    /// `func_8009728C`.
    pub fn room_request(&mut self, num: i8) -> bool {
        self.room_ctx.request(num)
    }

    /// `func_800973FC`: finishes a room load: the room's header, then the transition actors.
    /// True when no load is in flight.
    pub fn room_finish_load(&mut self) -> bool {
        if self.room_ctx.status != 1 {
            return true;
        }
        self.room_ctx.status = 0;
        self.room_ctx.cur.loaded = true;
        self.execute_room_commands();
        self.spawn_transition_actors();
        true
    }

    /// `Scene_ExecuteCommands` on the current room's header.
    fn execute_room_commands(&mut self) {
        let Some(room) = self.scene.as_ref().and_then(|s| s.room(self.room_ctx.cur.num)).cloned() else { return };
        // SCENE_CMD_ID_ROOM_BEHAVIOR, _ECHO_SETTINGS.
        let cur = &mut self.room_ctx.cur;
        cur.behavior_type1 = room.behavior[0];
        cur.behavior_type2 = room.behavior[1];
        cur.echo = room.echo;
        // SCENE_CMD_ID_ACTOR_LIST: spawned by the next Actor_UpdateAll.
        self.setup_actors = room.actors.clone();
        // SCENE_CMD_ID_OBJECT_LIST.
        if self.object_ctx.command_object_list(&room.objects) {
            self.kill_actors_without_objects();
        }
        // WaterBox_GetSurfaceImpl reads roomCtx.curRoom.num.
        self.col.water_room = self.room_ctx.cur.num.max(0) as u32;
    }

    /// `func_80097534`: Player is through: the previous room goes, and with it the actors
    /// that were only in it; the transition actors of the new pair spawn.
    pub fn room_change_done(&mut self) {
        self.room_ctx.prev = Room::EMPTY;
        self.kill_actors_outside_rooms();
        self.spawn_transition_actors();
        self.col.water_room = self.room_ctx.cur.num.max(0) as u32;
    }

    /// `Play_SetupRespawnPoint`.
    pub fn setup_respawn_point(&mut self, mode: usize, params: i16) {
        if let Some(pv) = self.player_view() {
            self.with_io(|io| io.setup_respawn_point(mode, params, pv.pos, pv.shape_yaw));
        }
    }

    /// `Play_TriggerVoidOut`.
    pub fn trigger_void_out(&mut self) {
        self.with_io(|io| io.trigger_void_out());
    }

    /// `Play_TriggerRespawn`.
    pub fn trigger_respawn(&mut self) {
        if let Some(pv) = self.player_view() {
            self.with_io(|io| io.trigger_respawn(pv.pos, pv.shape_yaw));
        }
    }

    /// `play->setupExitList`.
    pub fn exit_list(&self) -> &[u16] {
        self.scene.as_ref().map(|s| s.layer_data().exits.as_slice()).unwrap_or(&[])
    }

    /// `play->setupPathList` (`Scene_CommandPathList`); empty without a scene from the pack.
    pub fn setup_path_list(&self) -> &[crate::scene::Path] {
        self.scene.as_ref().map(|s| s.layer_data().paths.as_slice()).unwrap_or(&[])
    }

    /// `Play_SetupTransition`: which mode the type starts in.
    fn setup_transition(&mut self, ty: u8) {
        let tr = &mut self.transition;
        tr.ctx_type = ty;
        tr.mode = if is_instance_type(ty) {
            TRANS_MODE_INSTANCE_INIT
        } else {
            match ty {
                TRANS_TYPE_FILL_WHITE2 | TRANS_TYPE_FILL_WHITE => TRANS_MODE_FILL_WHITE_INIT,
                TRANS_TYPE_INSTANT => TRANS_MODE_INSTANT,
                TRANS_TYPE_FILL_BROWN => TRANS_MODE_FILL_BROWN_INIT,
                TRANS_TYPE_SANDSTORM_PERSIST => TRANS_MODE_SANDSTORM_INIT,
                TRANS_TYPE_SANDSTORM_END => TRANS_MODE_SANDSTORM_END_INIT,
                TRANS_TYPE_CS_BLACK_FILL => TRANS_MODE_CS_BLACK_FILL_INIT,
                _ => TRANS_MODE_INSTANCE_INIT,
            }
        };
    }

    /// The scene ends: `SET_NEXT_GAMESTATE(Play_Init)` with the next entrance.
    fn end_scene(&mut self) {
        self.save.entrance_index = self.transition.next_entrance_index;
        self.next_play_init = true;
    }

    /// The transition part of `Play_Update`, before the frame's actors update.
    pub(crate) fn update_transition(&mut self) {
        if self.transition.mode == TRANS_MODE_OFF && self.transition.trigger != TRANS_TRIGGER_OFF {
            self.transition.mode = TRANS_MODE_SETUP;
        }
        if self.transition.mode == TRANS_MODE_OFF {
            return;
        }
        if self.transition.mode == TRANS_MODE_SETUP {
            // (Fading out the BGM when the entrance doesn't continue it: no audio.)
            self.setup_transition(self.transition.ty);
        }
        match self.transition.mode {
            TRANS_MODE_INSTANCE_INIT => {
                let ty = self.transition.ctx_type;
                self.save.trans_wipe_speed = if ty == TRANS_TYPE_WIPE_FAST { 28 } else { 14 };
                self.save.trans_fade_duration = match ty {
                    TRANS_TYPE_FADE_BLACK_FAST | TRANS_TYPE_FADE_WHITE_FAST => 20,
                    TRANS_TYPE_FADE_BLACK_SLOW | TRANS_TYPE_FADE_WHITE_SLOW => 150,
                    TRANS_TYPE_FADE_WHITE_INSTANT => 2,
                    _ => 60,
                };
                let tr = &mut self.transition;
                tr.fade = TransitionFade::default();
                tr.fade.set_color(match ty {
                    TRANS_TYPE_FADE_WHITE | TRANS_TYPE_FADE_WHITE_FAST | TRANS_TYPE_FADE_WHITE_SLOW | TRANS_TYPE_FADE_WHITE_CS_DELAYED | TRANS_TYPE_FADE_WHITE_INSTANT => {
                        rgba8(160, 160, 160, 255)
                    }
                    TRANS_TYPE_FADE_GREEN => rgba8(140, 140, 100, 255),
                    TRANS_TYPE_FADE_BLUE => rgba8(70, 100, 110, 255),
                    _ => rgba8(0, 0, 0, 0),
                });
                tr.fade.set_type(if tr.trigger == TRANS_TRIGGER_END { 1 } else { 2 });
                tr.fade.start();
                // TRANS_TYPE_FADE_WHITE_CS_DELAYED waits, covering the screen, until a script's
                // cutsceneTransitionControl lets it run (its TRANSITION_FX 9).
                tr.mode = if ty == TRANS_TYPE_FADE_WHITE_CS_DELAYED { TRANS_MODE_INSTANCE_WAIT } else { TRANS_MODE_INSTANCE_RUNNING };
            }
            TRANS_MODE_INSTANCE_WAIT => {
                if self.save.cutscene_transition_control != 0 {
                    self.transition.mode = TRANS_MODE_INSTANCE_RUNNING;
                }
            }
            TRANS_MODE_INSTANCE_RUNNING => {
                if self.transition.fade.is_done {
                    if self.transition.trigger != TRANS_TRIGGER_END {
                        self.end_scene();
                    } else {
                        self.transition.mode = TRANS_MODE_OFF;
                    }
                    self.transition.trigger = TRANS_TRIGGER_OFF;
                } else {
                    let d = self.save.trans_fade_duration;
                    self.transition.fade.update(R_UPDATE_RATE, d);
                }
            }
            _ => {}
        }
        // The transitions Play_Update runs itself.
        let tr = &mut self.transition;
        match tr.mode {
            TRANS_MODE_FILL_WHITE_INIT | TRANS_MODE_FILL_BROWN_INIT => {
                tr.fill_timer = 0;
                let rgb = if tr.mode == TRANS_MODE_FILL_WHITE_INIT { [160, 160, 160] } else { [170, 160, 150] };
                if tr.trigger != TRANS_TRIGGER_END {
                    tr.screen_fill = Some([rgb[0], rgb[1], rgb[2], 0]);
                    tr.mode = TRANS_MODE_FILL_IN;
                } else {
                    tr.screen_fill = Some([rgb[0], rgb[1], rgb[2], 255]);
                    tr.mode = TRANS_MODE_FILL_OUT;
                }
            }
            TRANS_MODE_FILL_IN => {
                if let Some(f) = &mut tr.screen_fill {
                    f[3] = ((tr.fill_timer as f32 / 20.0) * 255.0) as u8;
                }
                if tr.fill_timer >= 20 {
                    tr.trigger = TRANS_TRIGGER_OFF;
                    tr.mode = TRANS_MODE_OFF;
                    self.end_scene();
                } else {
                    tr.fill_timer += 1;
                }
            }
            TRANS_MODE_FILL_OUT => {
                if let Some(f) = &mut tr.screen_fill {
                    f[3] = ((1.0 - tr.fill_timer as f32 / 20.0) * 255.0) as u8;
                }
                if tr.fill_timer >= 20 {
                    tr.trigger = TRANS_TRIGGER_OFF;
                    tr.mode = TRANS_MODE_OFF;
                    tr.screen_fill = None;
                } else {
                    tr.fill_timer += 1;
                }
            }
            TRANS_MODE_CS_BLACK_FILL_INIT => {
                tr.fill_timer = 0;
                tr.screen_fill = Some([0, 0, 0, 255]);
                tr.mode = TRANS_MODE_CS_BLACK_FILL;
            }
            TRANS_MODE_CS_BLACK_FILL => {
                // The script's TRANSITION_FX 12 lowers cutsceneTransitionControl from 255; the
                // fill follows it, and the transition ends at 100 or less, the fill left as it is.
                let c = self.save.cutscene_transition_control;
                if c != 0 {
                    if let Some(f) = &mut tr.screen_fill {
                        f[3] = c;
                    }
                    if c <= 100 {
                        tr.trigger = TRANS_TRIGGER_OFF;
                        tr.mode = TRANS_MODE_OFF;
                    }
                }
            }
            TRANS_MODE_INSTANT | TRANS_MODE_SANDSTORM_INIT | TRANS_MODE_SANDSTORM_END_INIT => {
                // Instant; the sandstorm isn't ported and ends at once too.
                let leaving = tr.trigger != TRANS_TRIGGER_END;
                tr.trigger = TRANS_TRIGGER_OFF;
                tr.mode = TRANS_MODE_OFF;
                if leaving {
                    self.end_scene();
                }
            }
            _ => {}
        }
    }

    /// What covers the screen this frame: `Play_Draw` draws the environment's fill
    /// (`envCtx.fillScreen`, a cutscene's `TRANSITION_FX`) and the transition's fade over it, as
    /// one fill (two full-screen blends compose into one).
    pub fn screen_fill(&self) -> Option<[u8; 4]> {
        let tr = &self.transition;
        // A state from Play_Init that hasn't run a frame yet: its fade starts in its first
        // Play_Update, and until then the screen is still what the last state left.
        if !self.updated && tr.trigger != TRANS_TRIGGER_OFF {
            return self.pre_update_fill;
        }
        let env = tr.screen_fill.filter(|f| f[3] > 0);
        let fade = (tr.mode == TRANS_MODE_INSTANCE_RUNNING || tr.mode == TRANS_MODE_INSTANCE_INIT || tr.mode == TRANS_MODE_INSTANCE_WAIT).then(|| tr.fade.fill()).flatten();
        match (env, fade) {
            (Some(e), Some(f)) => Some(compose_fill(e, f)),
            (e, f) => f.or(e),
        }
    }
}

/// `top` blended over `bottom`, both full-screen fills, as one fill: the same pixels as drawing
/// `bottom` then `top`.
fn compose_fill(bottom: [u8; 4], top: [u8; 4]) -> [u8; 4] {
    let (ab, at) = (bottom[3] as f32 / 255.0, top[3] as f32 / 255.0);
    let a = at + ab * (1.0 - at);
    if a <= 0.0 {
        return [0, 0, 0, 0];
    }
    let c = |k: usize| ((top[k] as f32 * at + bottom[k] as f32 * ab * (1.0 - at)) / a).round() as u8;
    [c(0), c(1), c(2), (a * 255.0).round() as u8]
}

#[cfg(test)]
mod tests {
    use super::compose_fill;

    #[test]
    fn a_fade_over_the_cutscenes_fill_covers_as_both_drawn() {
        // A full fill stays full whatever fades over it (the nightmare's black under the
        // entrance's FADE_BLACK_FAST).
        assert_eq!(compose_fill([0, 0, 0, 255], [0, 0, 0, 26]), [0, 0, 0, 255]);
        // Half over half: 1 - 0.5 * 0.5 of the scene covered.
        assert_eq!(compose_fill([0, 0, 0, 128], [0, 0, 0, 128]), [0, 0, 0, 192]);
        // White over black, both opaque: the white.
        assert_eq!(compose_fill([0, 0, 0, 255], [160, 160, 160, 255]), [160, 160, 160, 255]);
        assert_eq!(compose_fill([0, 0, 0, 0], [0, 0, 0, 0]), [0, 0, 0, 0]);
    }
}
