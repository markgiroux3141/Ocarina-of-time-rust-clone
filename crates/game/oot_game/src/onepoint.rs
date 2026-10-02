//! One-point cutscenes (`z_onepointdemo.c`): short camera shots that actors and Player start
//! (`OnePointCutscene_Init`), each on a sub camera queued in front of the camera it interrupts.
//!
//! A one-point cutscene's camera is a sub camera (`Play_CreateSubCamera`) with a `csId`, a timer
//! and a target. `OnePointCutscene_SetInfo` picks its setting and data by `csId`: most play a
//! list of `OnePointCsFull` keyframes on `CAM_SET_CS_C` (`Camera_Unique9`), the crawlspace's
//! exits a spline on `CAM_SET_CS_3` (`Camera_Demo9`), the attention cutscenes start on
//! `CAM_SET_CS_ATTENTION` (`Camera_Demo5`, which picks keyframes by where the target is), and a
//! few hold a fixed shot (`CAM_SET_FREE2`, `Camera_Unique6`). The cameras form a queue through
//! `parentCamId` and `childCamId`: the camera a cutscene interrupts is its parent, and a
//! cutscene already in front of it becomes its child and waits. When a camera's timer runs out
//! (`Camera_Finish`, at the end of `Play_Draw`), its parent becomes active again; back at the
//! main camera, Player's cutscene mode ends.
//!
//! The keyframe and spline tables are statics the game writes into as it runs (a case's
//! timer, the targets it computes from the view), so the play state keeps them
//! (`OnePointStatics`) from the pack's copy of the ROM's (`CameraData::onepoint`), and they
//! carry over scene changes, as the code segment's statics do.

use glam::Vec3;

use crate::actor_ctx::{ACTORCAT_DOOR, ActorHandle};
use crate::camera::{
    CAM_ID_MAIN, CAM_ID_NONE, CAM_ID_SUB_FIRST, CAM_MODE_FOLLOWBOOMERANG, CAM_MODE_NORMAL, CAM_SET_CS_ATTENTION, CAM_STAT_ACTIVE, CAM_STAT_UNK3, CAM_STAT_WAIT, CamActor, CamRequest, KeyFramesRef,
    OnePointCsFull, OnePointData, VecSph, cam_binang_to_deg, diff_to_sph_geo, sph_geo_add, sph_geo_to_vec3,
};
use crate::cutscene::CutsceneCameraPoint;
use crate::play::PlayState;

// CAM_SET_* (z64camera.h) the one-point cutscenes use (the pack's test checks them against
// the enum).
pub const CAM_SET_FREE2: i16 = 0x22;
pub const CAM_SET_CS_3: i16 = 0x2A;
pub const CAM_SET_CS_C: i16 = 0x3C;
pub const CAM_SET_TURN_AROUND: i16 = crate::camera::CAM_SET_TURN_AROUND;

/// `PLAYER_STATE1_29`, `PLAYER_STATE1_27` (`z64player.h`).
const PLAYER_STATE1_27: u32 = 1 << 27;
const PLAYER_STATE1_29: u32 = 1 << 29;

/// The one-point cutscenes' statics: `z_onepointdemo.c`'s (its data file's tables,
/// `sDisableAttention`, `sUnused`, `sPrevFrameCs1100`) and the ones `Camera_Unique9` and
/// `Camera_Demo5` keep in `z_camera_data.c` (`D_8011D3AC`, `sDemo5PrevAction12Frame`,
/// `sDemo5PrevSfxFrame`, `Camera_Demo5`'s keyframe tables).
#[derive(Debug, Clone, PartialEq)]
pub struct OnePointStatics {
    pub keyframes: Vec<Vec<OnePointCsFull>>,
    pub keyframe_names: Vec<String>,
    pub points: Vec<Vec<CutsceneCameraPoint>>,
    pub point_names: Vec<String>,
    pub shorts: Vec<(String, i16)>,
    /// `sDisableAttention`, `sUnused`, `sPrevFrameCs1100`.
    pub disable_attention: bool,
    pub unused: i16,
    pub prev_frame_cs1100: i32,
    /// `D_8011D3AC` (`func_8005B198`): the category the attention camera last attended, or a
    /// keyframe's `0x8X`.
    pub d_8011d3ac: i32,
    /// `sDemo5PrevAction12Frame`, `sDemo5PrevSfxFrame`.
    pub demo5_prev_action12_frame: i32,
    pub demo5_prev_sfx_frame: i32,
    /// Where a write to a missing row goes (`kf`).
    scratch: OnePointCsFull,
}

impl OnePointStatics {
    /// The statics as the ROM starts them.
    pub fn new(d: &OnePointData) -> OnePointStatics {
        OnePointStatics {
            keyframes: d.keyframes.iter().map(|(_, v)| v.clone()).collect(),
            keyframe_names: d.keyframes.iter().map(|(n, _)| n.clone()).collect(),
            points: d.points.iter().map(|(_, v)| v.clone()).collect(),
            point_names: d.points.iter().map(|(n, _)| n.clone()).collect(),
            shorts: d.shorts.clone(),
            disable_attention: false,
            unused: -1,
            prev_frame_cs1100: -4096,
            d_8011d3ac: -1,
            demo5_prev_action12_frame: -16,
            demo5_prev_sfx_frame: -200,
            scratch: OnePointCsFull::default(),
        }
    }

    /// The keyframe table `name` (`D_801208EC`); `usize::MAX` (no rows) for a name the pack
    /// doesn't have.
    pub fn table(&self, name: &str) -> usize {
        self.keyframe_names.iter().position(|n| n == name).unwrap_or(usize::MAX)
    }

    /// The point list `name` (`D_80120308`).
    pub fn point_list(&self, name: &str) -> Option<usize> {
        self.point_names.iter().position(|n| n == name)
    }

    /// The `s16` `name` (`D_8012042C`).
    pub fn short(&self, name: &str) -> i16 {
        self.shorts.iter().find(|(n, _)| n == name).map(|(_, v)| *v).unwrap_or(0)
    }

    /// The row a keyframe pointer points at (zeros past the table, where the C would read on).
    pub fn key_frame(&self, r: KeyFramesRef) -> OnePointCsFull {
        self.keyframes.get(r.table).and_then(|t| t.get(r.start)).copied().unwrap_or_default()
    }

    /// Row `i` of table `t`, to write (a missing row is an import error, logged; the write
    /// then goes to a scratch row nothing reads).
    pub fn kf(&mut self, t: usize, i: usize) -> &mut OnePointCsFull {
        if self.keyframes.get(t).is_some_and(|v| i < v.len()) {
            return &mut self.keyframes[t][i];
        }
        log::error!("onepoint: no keyframe table {t} row {i}");
        &mut self.scratch
    }

    /// The number of rows of table `t` (`ARRAY_COUNT`).
    pub fn len(&self, t: usize) -> i32 {
        self.keyframes.get(t).map(|v| v.len() as i32).unwrap_or(0)
    }

    /// The points of list `i` (`Camera_Demo9` reads them through `onePointCamData`'s pointers).
    pub fn points_of(&self, i: Option<usize>) -> &[CutsceneCameraPoint] {
        i.and_then(|i| self.points.get(i)).map(|v| v.as_slice()).unwrap_or(&[])
    }
}

impl PlayState {
    /// What the camera functions read of actor `h` (`CamActor`).
    pub fn cam_actor(&self, h: ActorHandle) -> Option<CamActor> {
        let a = self.actors.actor(h)?;
        Some(self.cam_actor_of(h, a))
    }

    /// `CamActor` for actor `h` given its base: an actor starting a one-point cutscene from its
    /// own update is out of the arena (docs/adr/0007-actor-ownership.md), so it passes itself.
    pub fn cam_actor_of(&self, h: ActorHandle, a: &crate::actor::Actor) -> CamActor {
        let rot = |r: crate::actor::Rot| [r.x, r.y, r.z];
        let door_yaw = self.func_800c0d34_of(a);
        CamActor {
            handle: h,
            category: a.category,
            alive: !a.killed,
            focus_pos: a.focus_pos,
            focus_rot: rot(a.focus_rot),
            world_pos: a.world_pos,
            world_rot: rot(a.world_rot),
            shape_rot: rot(a.shape_rot),
            screen_pos: crate::target::actor_screen_pos(self.view_proj, a),
            door_yaw,
        }
    }

    /// `func_800C0D34` (`z_play.c`): a door's yaw facing its front room, when it's a transition
    /// actor between two rooms.
    pub fn func_800c0d34(&self, h: ActorHandle) -> Option<i16> {
        self.func_800c0d34_of(self.actors.actor(h)?)
    }

    fn func_800c0d34_of(&self, a: &crate::actor::Actor) -> Option<i16> {
        if a.category != ACTORCAT_DOOR {
            return None;
        }
        // GET_TRANSITION_ACTOR_INDEX(actor): params >> 10.
        let t = self.transi_actors.get((a.params as u16 >> 10) as usize)?;
        let front_room = t.sides[0].0;
        if front_room == t.sides[1].0 {
            return None;
        }
        Some(if front_room == a.room { a.shape_rot.y } else { a.shape_rot.y.wrapping_add(i16::MIN) })
    }

    /// `func_800C0808` (`z_play.c`): `Camera_InitPlayerSettings` with Player (or the actor given
    /// for him), then `Camera_ChangeSetting`.
    pub fn func_800c0808(&mut self, cam_id: i16, player: Option<ActorHandle>, setting: i16) -> i16 {
        let id = if cam_id == CAM_ID_NONE { self.active_cam_id } else { cam_id };
        let pv = player.and_then(|h| self.player_view_of(h));
        let d = self.data.clone();
        let is_main = id == CAM_ID_MAIN;
        let room = self.cam_room();
        let Some(c) = self.camera_mut(id) else { return -99 };
        if let Some(pv) = pv {
            let flags = c.init_player_settings(&d.camera, &pv, is_main, room);
            self.cam_globals.interface_flags = flags;
        }
        self.camera_mut(id).map(|c| c.change_setting(&d.camera, setting)).unwrap_or(-99)
    }

    /// `Play_SetCameraRoll`.
    pub fn play_set_camera_roll(&mut self, cam_id: i16, roll: i16) {
        if let Some(c) = self.camera_mut(cam_id) {
            c.roll = roll;
        }
    }

    /// `func_800C08AC` (`z_play.c`): back to the main camera, clearing `cam_id` and any other sub
    /// camera; with a time, through the 1020 cutscene's return.
    pub fn func_800c08ac(&mut self, cam_id: i16, arg2: i16) {
        let id = if cam_id == CAM_ID_NONE { self.active_cam_id } else { cam_id };
        self.clear_camera(id);
        for i in CAM_ID_SUB_FIRST..crate::camera::NUM_CAMS as i16 {
            if self.camera(i).is_some() {
                log::error!("camera control: error: return to main, other camera left. {i} cleared!!");
                self.clear_camera(i);
            }
        }
        if arg2 <= 0 {
            self.change_camera_status(CAM_ID_MAIN, CAM_STAT_ACTIVE);
            self.game_camera.child_cam_id = CAM_ID_MAIN;
            self.game_camera.parent_cam_id = CAM_ID_MAIN;
        } else {
            self.onepoint_cutscene_init(1020, arg2, None, CAM_ID_MAIN);
        }
    }

    fn cam_child(&self, id: i16) -> i16 {
        self.camera(id).map(|c| c.child_cam_id).unwrap_or(CAM_ID_MAIN)
    }
    fn cam_parent(&self, id: i16) -> i16 {
        self.camera(id).map(|c| c.parent_cam_id).unwrap_or(CAM_ID_MAIN)
    }

    /// `OnePointCutscene_SetAsChild`: `new` goes in front of `parent`; returns the camera that
    /// was there.
    pub fn onepoint_set_as_child(&mut self, new: i16, parent: i16) -> i16 {
        let prev = self.cam_child(parent);
        if let Some(c) = self.camera_mut(new) {
            c.parent_cam_id = parent;
        }
        if let Some(c) = self.camera_mut(parent) {
            c.child_cam_id = new;
        }
        prev
    }

    /// `OnePointCutscene_RemoveCamera`: takes `sub` out of the queue and clears it; returns its
    /// parent if it was the active camera, else `CAM_ID_NONE`.
    pub fn onepoint_remove_camera(&mut self, sub: i16) -> i16 {
        let (child, parent) = (self.cam_child(sub), self.cam_parent(sub));
        if self.cam_parent(child) == sub
            && let Some(c) = self.camera_mut(child)
        {
            c.parent_cam_id = parent;
        }
        if self.cam_child(parent) == sub
            && let Some(c) = self.camera_mut(parent)
        {
            c.child_cam_id = child;
        }
        let next = if self.active_cam_id == sub { parent } else { CAM_ID_NONE };
        if let Some(c) = self.camera_mut(sub) {
            c.parent_cam_id = CAM_ID_MAIN;
            c.child_cam_id = CAM_ID_MAIN;
            c.timer = -1;
        }
        self.clear_camera(sub);
        next
    }

    /// `OnePointCutscene_Init`: a sub camera for one-point cutscene `cs_id`, lasting `timer`,
    /// about `actor`, in front of `parent` (`CAM_ID_NONE`: the active camera) in the queue; the
    /// lower priority cutscenes (`csId / 100`) in front of it go. Returns the camera, or
    /// `CAM_ID_NONE` when there's none free.
    pub fn onepoint_cutscene_init(&mut self, cs_id: i16, timer: i16, actor: Option<CamActor>, parent: i16) -> i16 {
        let parent = if parent == CAM_ID_NONE { self.active_cam_id } else { parent };
        let sub = self.create_sub_camera();
        if sub == CAM_ID_NONE {
            log::error!("onepoint demo: error: too many cameras ... give up! type={cs_id}");
            return CAM_ID_NONE;
        }
        // Inserts the cutscene camera into the queue in front of the parent.
        let child = self.cam_child(parent);
        let mut sub_status = CAM_STAT_ACTIVE;
        if child >= CAM_ID_SUB_FIRST {
            self.onepoint_set_as_child(child, sub);
            sub_status = CAM_STAT_WAIT;
        } else {
            crate::interface::change_alpha(&mut self.save, 2);
        }
        self.onepoint_set_as_child(sub, parent);
        let view = self.view;
        if let Some(c) = self.camera_mut(sub) {
            c.timer = timer;
            c.target = actor.map(|a| a.handle);
            c.at = view.at;
            c.eye = view.eye;
            c.fov = view.fov;
            c.cs_id = cs_id;
        }
        self.change_camera_status(parent, if parent == CAM_ID_MAIN { CAM_STAT_UNK3 } else { CAM_STAT_WAIT });
        self.onepoint_cutscene_set_info(sub, cs_id, actor, timer);
        self.change_camera_status(sub, sub_status);
        log::debug!("onepoint camera[{sub}]: {cs_id}, timer {timer}, parent {parent}");

        // Removes all lower priority cutscenes in front of this one from the queue.
        let mut cur = sub;
        let mut next = self.cam_child(sub);
        while next >= CAM_ID_SUB_FIRST {
            let next_cs = self.camera(next).map(|c| c.cs_id).unwrap_or(0);
            let this_cs = self.camera(sub).map(|c| c.cs_id).unwrap_or(0);
            if next_cs / 100 < this_cs / 100 {
                log::debug!("onepointdemo camera[{next}]: killed 'coz low priority ({next_cs} < {this_cs})");
                if next_cs != 5010 {
                    next = self.onepoint_remove_camera(next);
                    if next != CAM_ID_NONE {
                        self.change_camera_status(next, CAM_STAT_ACTIVE);
                    }
                } else {
                    cur = next;
                    self.onepoint_end_cutscene(next);
                }
            } else {
                cur = next;
            }
            next = self.cam_child(cur);
        }
        sub
    }

    /// `OnePointCutscene_EndCutscene`: the camera's timer to 0 (5 for an attention cutscene),
    /// so `Camera_Finish` ends it.
    pub fn onepoint_end_cutscene(&mut self, sub: i16) -> i16 {
        let sub = if sub == CAM_ID_NONE { self.active_cam_id } else { sub };
        if let Some(c) = self.camera_mut(sub) {
            log::debug!("onepointdemo camera[{sub}]: delete timer={} next={}", c.timer, c.parent_cam_id);
            c.timer = if c.cs_id == 5010 { 5 } else { 0 };
        }
        sub
    }

    /// `OnePointCutscene_Attention`: an attention cutscene (5010) on `actor`, queued after the
    /// attention cutscenes on actors of higher categories; none for a category already queued.
    pub fn onepoint_attention(&mut self, actor: CamActor) -> i16 {
        if self.onepoint.disable_attention {
            log::debug!("actor attention demo camera: canceled by other camera");
            return CAM_ID_NONE;
        }
        self.onepoint.unused = -1;
        if self.game_camera.mode == CAM_MODE_FOLLOWBOOMERANG {
            let d = self.data.clone();
            self.game_camera.change_mode(&d.camera, CAM_MODE_NORMAL);
            self.camera_sfx();
        }
        let category = actor.category;
        // The first attention cutscene on an actor of a lower category, or the first other
        // cutscene after at least one attention cutscene.
        let mut parent = CAM_ID_MAIN;
        let mut last_higher_cat: i32 = -1;
        loop {
            let child = self.cam_child(parent);
            if child == CAM_ID_MAIN {
                break;
            }
            let Some(c) = self.camera(child) else {
                // parentCam = NULL: the loop ends with it.
                break;
            };
            parent = child;
            if c.setting != CAM_SET_CS_ATTENTION {
                if last_higher_cat == -1 {
                    continue;
                }
                break;
            }
            let target_cat = c.target.and_then(|t| self.actors.actor(t)).map(|a| a.category as i32).unwrap_or(-1);
            if category as i32 > target_cat {
                break;
            }
            last_higher_cat = target_cat;
        }
        let parent_cam_id = if last_higher_cat == -1 { CAM_ID_MAIN } else { parent };
        use crate::actor_ctx::*;
        let timer = match category {
            ACTORCAT_SWITCH | ACTORCAT_BG | ACTORCAT_PLAYER | ACTORCAT_PROP | ACTORCAT_DOOR => 30,
            ACTORCAT_NPC | ACTORCAT_ITEMACTION | ACTORCAT_CHEST => 100,
            _ => {
                log::debug!("actor attention demo camera: {}: unkown part of actor {category}", self.gameplay_frames);
                30
            }
        };
        if category as i32 == last_higher_cat {
            return CAM_ID_NONE;
        }
        let sub = self.onepoint_cutscene_init(5010, timer, Some(actor), parent_cam_id);
        if sub == CAM_ID_NONE {
            log::error!("actor attention demo: give up!");
            return CAM_ID_NONE;
        }
        if let Some(c) = self.camera_mut(sub) {
            c.data1 = crate::audio::sfx::NA_SE_SY_CORRECT_CHIME;
        }
        sub
    }

    /// `OnePointCutscene_AttentionSetSfx`.
    pub fn onepoint_attention_set_sfx(&mut self, actor: CamActor, sfx_id: u16) -> i16 {
        let sub = self.onepoint_attention(actor);
        if sub != CAM_ID_NONE
            && let Some(c) = self.camera_mut(sub)
        {
            c.data1 = sfx_id;
        }
        sub
    }

    /// `OnePointCutscene_CheckForCategory`: an attention cutscene queued on an actor of
    /// `category`.
    pub fn onepoint_check_for_category(&self, category: usize) -> bool {
        let mut cam = CAM_ID_MAIN;
        loop {
            let child = self.cam_child(cam);
            if child == CAM_ID_MAIN {
                return false;
            }
            let Some(c) = self.camera(child) else { return false };
            if c.setting != CAM_SET_CS_ATTENTION {
                return false;
            }
            if c.target.and_then(|t| self.actors.actor(t)).map(|a| a.category) == Some(category) {
                return true;
            }
            cam = child;
        }
    }

    /// `func_8005B198`: `D_8011D3AC`.
    pub fn func_8005b198(&self) -> i32 {
        self.onepoint.d_8011d3ac
    }

    /// `Camera_Finish` (`z_camera.c`, at the end of `Play_Draw` for the active camera): when the
    /// camera's timer is 0, its parent becomes active again and the camera goes; back at the
    /// main camera, Player's cutscene ends.
    pub fn camera_finish(&mut self, id: i16) {
        let Some(c) = self.camera(id) else { return };
        if c.timer != 0 {
            return;
        }
        let (parent, child, cs_id, cam_id) = (c.parent_cam_id, c.child_cam_id, c.cs_id, c.cam_id);
        self.change_camera_status(parent, CAM_STAT_ACTIVE);
        if parent == CAM_ID_MAIN && cs_id != 0 {
            let cs_mode = self.player.and_then(|h| self.actors.get(h)).and_then(|p| p.as_player()).map(|p| p.cs_mode()).unwrap_or(0);
            if let Some(ph) = self.player {
                if let Some(a) = self.actors.actor_mut(ph) {
                    a.freeze_timer = 0;
                }
                if let Some(p) = self.actors.get_mut(ph).and_then(|p| p.as_player_mut()) {
                    p.change_state_flags1(0, PLAYER_STATE1_29);
                }
                if cs_mode != 0 {
                    self.func_8002df54(Some(ph), 7);
                    log::debug!("camera: player demo end!!");
                }
            }
            self.game_camera.unk_14c |= 8;
        }
        if self.cam_parent(child) == cam_id
            && let Some(c) = self.camera_mut(child)
        {
            c.parent_cam_id = parent;
        }
        if self.cam_child(parent) == cam_id
            && let Some(c) = self.camera_mut(parent)
        {
            c.child_cam_id = child;
        }
        if self.camera(parent).is_some_and(|c| c.cam_id == CAM_ID_MAIN) {
            self.game_camera.reset_anim();
        }
        if let Some(c) = self.camera_mut(id) {
            c.child_cam_id = CAM_ID_MAIN;
            c.parent_cam_id = CAM_ID_MAIN;
            c.timer = -1;
        }
        // play->envCtx.fillScreen = false.
        self.transition.screen_fill = None;
        self.clear_camera(cam_id);
    }

    /// Applies what camera `id`'s update did to the rest of play (`CamRequest`), in order.
    pub fn apply_cam_requests(&mut self, id: i16) {
        let Some(reqs) = self.camera_mut(id).map(|c| std::mem::take(&mut c.requests)) else { return };
        for r in reqs {
            match r {
                CamRequest::PlayerCsMode { actor, mode, door } => {
                    if door {
                        self.func_8002df54(actor, mode);
                    } else {
                        self.func_8002df38(actor, mode);
                    }
                }
                CamRequest::PlayerPos { x, z, y } => {
                    if let Some(a) = self.player.and_then(|h| self.actors.actor_mut(h)) {
                        a.world_pos.x = x;
                        a.world_pos.z = z;
                        if let Some(y) = y {
                            a.world_pos.y = y;
                        }
                    }
                }
                CamRequest::PlayerFreeze { timer } => {
                    if let Some(ph) = self.player {
                        if let Some(p) = self.actors.get_mut(ph).and_then(|p| p.as_player_mut()) {
                            p.change_state_flags1(PLAYER_STATE1_29, 0);
                        }
                        if let Some(a) = self.actors.actor_mut(ph) {
                            a.freeze_timer = timer as u16;
                        }
                    }
                }
                CamRequest::CopyTo { dest, at, eye, fov, roll } => {
                    let (d, player_pos) = (self.data.clone(), self.player.and_then(|h| self.actors.actor(h)).map(|a| a.world_pos).unwrap_or(Vec3::ZERO));
                    if let Some(c) = self.camera_mut(dest) {
                        c.copy_from_values(&d.camera, at, eye, fov, roll, player_pos);
                    }
                }
                CamRequest::ChangeModeFlags { cam, mode, flags } => {
                    let d = self.data.clone();
                    if let Some(c) = self.camera_mut(cam) {
                        c.change_mode_flags(&d.camera, mode, flags);
                    }
                    self.camera_sfx();
                }
                CamRequest::OnePointInit { cs_id, timer, parent } => {
                    self.onepoint_cutscene_init(cs_id, timer, None, parent);
                }
            }
        }
    }

    /// `OnePointCutscene_SetInfo`: the camera's setting and data for `cs_id`.
    ///
    /// Ported: the cutscenes of the scenes played so far and Phase 6's Deku Tree, and the
    /// generic ones Player, `z_play.c` and the cameras start (1000, 1010, 1020, 1030, 1100, 3010,
    /// 3020, 3040, 3140, 4500, 4510, 5000, 5010, 5110, 5120, 9500, 9601, 9602, 9806, 9908). The
    /// others are logged as the C logs an unknown number (the camera keeps `Camera_Init`'s
    /// `CAM_SET_FREE0`).
    pub fn onepoint_cutscene_set_info(&mut self, sub: i16, cs_id: i16, actor: Option<CamActor>, timer: i16) -> i32 {
        let child = self.cam_child(sub);
        let main = self.game_camera.clone();
        let player = self.player;
        let view = self.view;
        let frames = self.gameplay_frames;
        let set_cs_info = |play: &mut PlayState, name: &str, cnt: i32| {
            let t = play.onepoint.table(name);
            if let Some(c) = play.camera_mut(sub) {
                c.cs_info.key_frames = Some(KeyFramesRef { table: t, start: 0 });
                c.cs_info.key_frame_cnt = cnt;
            }
        };
        match cs_id {
            1020 => {
                let timer = timer.max(20);
                let t = self.onepoint.table("D_801208EC");
                {
                    let k = self.onepoint.kf(t, 0);
                    k.at_target_init = view.at;
                    k.eye_target_init = view.eye;
                    k.fov_target_init = view.fov;
                }
                let k = self.onepoint.kf(t, 1);
                k.at_target_init = main.at;
                k.eye_target_init = main.eye;
                k.fov_target_init = main.fov;
                k.timer_init = timer - 1;
                k.lerp_step_scale = 1.0 / (0.5 * timer as f32);
                if let Some(c) = self.camera_mut(sub) {
                    c.timer = timer + 1;
                }
                set_cs_info(self, "D_801208EC", 3);
                self.func_800c0808(sub, player, CAM_SET_CS_C);
            }
            1030 => {
                let t = self.onepoint.table("D_80120964");
                {
                    let k = self.onepoint.kf(t, 0);
                    k.at_target_init = view.at;
                    k.eye_target_init = view.eye;
                    k.fov_target_init = view.fov;
                }
                let sp_d0 = diff_to_sph_geo(main.at, main.eye);
                let k = self.onepoint.kf(t, 1);
                k.eye_target_init.y = cam_binang_to_deg(sp_d0.yaw);
                k.timer_init = timer - 1;
                set_cs_info(self, "D_80120964", 2);
                self.func_800c0808(sub, player, CAM_SET_CS_C);
            }
            5000 => {
                let Some(a) = actor.map(|a| a.focus_pos) else { return 0 };
                let t = self.onepoint.table("D_801209B4");
                {
                    self.onepoint.kf(t, 0).at_target_init = view.at;
                    self.onepoint.kf(t, 1).at_target_init = view.at;
                    self.onepoint.kf(t, 0).eye_target_init = view.eye;
                    self.onepoint.kf(t, 0).fov_target_init = view.fov;
                    self.onepoint.kf(t, 2).fov_target_init = view.fov;
                }
                let mut sp_d0 = diff_to_sph_geo(a, main.at);
                sp_d0.r = main.dist;
                let at1 = self.onepoint.kf(t, 1).at_target_init;
                let k = self.onepoint.kf(t, 1);
                k.eye_target_init = sph_geo_add(at1, sp_d0);
                k.at_target_init.y += 20.0;
                set_cs_info(self, "D_801209B4", 4);
                self.func_800c0808(sub, player, CAM_SET_CS_C);
            }
            5010 => {
                self.func_800c0808(sub, player, CAM_SET_CS_ATTENTION);
                self.camera_set_at_eye(sub, main.at, main.eye);
                if let Some(c) = self.camera_mut(sub) {
                    c.roll = 0;
                }
            }
            9500 => {
                set_cs_info(self, "D_80120A54", 3);
                self.func_800c0808(sub, player, CAM_SET_CS_C);
            }
            4510 => {
                let Some(a) = actor.map(|a| a.world_pos) else { return 0 };
                let py = player.and_then(|h| self.actors.actor(h)).map(|a| a.world_pos.y).unwrap_or(0.0);
                let t = self.onepoint.table("D_8012133C");
                let k = self.onepoint.kf(t, 0);
                k.eye_target_init = a;
                k.eye_target_init.y = py + 40.0;
                self.func_8002df54(None, 8);
                set_cs_info(self, "D_8012133C", 3);
                self.func_800c0808(sub, player, CAM_SET_CS_C);
            }
            4500 => {
                let Some((focus, focus_rot)) = actor.map(|a| (a.focus_pos, a.focus_rot)) else { return 0 };
                let mut sp_c0 = focus;
                // OnePointCutscene_RaycastDown: BgCheck_EntityRaycastDown3.
                sp_c0.y = self.col.entity_raycast_down(sp_c0).0 + 40.0;
                let sp_d0 = VecSph { r: 150.0, yaw: focus_rot[1], pitch: 0x3E8 };
                let sp_b4 = sph_geo_add(sp_c0, sp_d0);
                self.camera_change_setting(sub, CAM_SET_FREE2);
                self.camera_set_at_eye(sub, sp_c0, sp_b4);
                self.func_8002df54(None, 8);
                let sub_child = if let Some(c) = self.camera_mut(sub) {
                    c.roll = 0;
                    c.fov = 50.0;
                    c.child_cam_id
                } else {
                    CAM_ID_MAIN
                };
                if sub_child != CAM_ID_MAIN {
                    self.onepoint_end_cutscene(sub_child);
                }
            }
            1010 => {
                let Some(cc) = self.camera(child).cloned() else { return 0 };
                self.camera_change_setting(sub, CAM_SET_FREE2);
                self.camera_set_at_eye(sub, cc.at, cc.eye);
                self.camera_set_fov(sub, cc.fov);
                self.play_set_camera_roll(sub, cc.roll);
            }
            9601 | 9602 => {
                self.camera_change_setting(sub, CAM_SET_CS_3);
                let prev = main.prev_setting;
                self.camera_change_setting(CAM_ID_MAIN, prev);
                let (at, eye) = if cs_id == 9601 { ("D_80120308", "D_80120398") } else { ("D_80120308", "D_80120434") };
                let action = self.onepoint.short("D_80120430") | 0x1000;
                let init_timer = self.onepoint.short("D_8012042C");
                let (at, eye) = (self.onepoint.point_list(at), self.onepoint.point_list(eye));
                if let Some(c) = self.camera_mut(sub) {
                    c.set_cs_cam_points(action, init_timer, at, eye);
                }
            }
            3040 => {
                self.func_8002df54(None, 8);
                let t = self.onepoint.table("D_8012151C");
                self.onepoint.kf(t, 0).timer_init = timer - 1;
                set_cs_info(self, "D_8012151C", 2);
                self.func_800c0808(sub, player, CAM_SET_CS_C);
            }
            3020 => {
                let t = self.onepoint.table("D_8012156C");
                self.onepoint.kf(t, 1).timer_init = timer - 1;
                if frames & 1 != 0 {
                    for i in 0..2 {
                        let k = self.onepoint.kf(t, i);
                        k.at_target_init.x = -k.at_target_init.x;
                        k.eye_target_init.x = -k.eye_target_init.x;
                    }
                }
                let temp_rand = self.rand.zero_one() * 15.0;
                self.onepoint.kf(t, 0).eye_target_init.x += temp_rand;
                self.onepoint.kf(t, 1).eye_target_init.x += temp_rand;
                set_cs_info(self, "D_8012156C", 2);
                self.func_800c0808(sub, player, CAM_SET_CS_C);
                self.func_8002df54(None, 8);
            }
            3010 => {
                let t = self.onepoint.table("D_801215BC");
                self.onepoint.kf(t, 0).timer_init = timer;
                set_cs_info(self, "D_801215BC", 1);
                self.func_800c0808(sub, player, CAM_SET_CS_C);
            }
            3140 => {
                let t = self.onepoint.table("D_80121C24");
                let k = self.onepoint.kf(t, 0);
                k.at_target_init = view.at;
                k.eye_target_init = view.eye;
                k.fov_target_init = view.fov;
                set_cs_info(self, "D_80121C24", 7);
                self.func_800c0808(sub, player, CAM_SET_CS_C);
            }
            1100 => {
                let temp_diff = frames as i32 - self.onepoint.prev_frame_cs1100;
                if !(-3600..=3600).contains(&temp_diff) {
                    set_cs_info(self, "D_80123074", 5);
                } else {
                    if frames & 1 != 0 {
                        let t = self.onepoint.table("D_8012313C");
                        let k = self.onepoint.kf(t, 0);
                        k.roll_target_init = k.roll_target_init.wrapping_neg();
                        k.at_target_init.y = -k.at_target_init.y;
                        k.eye_target_init.y = -k.eye_target_init.y;
                        let k = self.onepoint.kf(t, 1);
                        k.at_target_init.y = -k.at_target_init.y;
                    }
                    set_cs_info(self, "D_8012313C", 3);
                }
                self.func_800c0808(sub, player, CAM_SET_CS_C);
                self.onepoint.prev_frame_cs1100 = frames as i32;
            }
            9806 => {
                if let Some(c) = self.camera_mut(sub) {
                    c.timer = -99;
                }
                if self.cam_is_not_fixed() {
                    self.func_800c0808(sub, player, CAM_SET_TURN_AROUND);
                    if let Some(c) = self.camera_mut(sub) {
                        c.data2 = 0xC;
                    }
                } else {
                    self.copy_camera(sub, CAM_ID_MAIN);
                    self.camera_change_setting(sub, CAM_SET_FREE2);
                }
            }
            9908 => {
                if self.cam_is_not_fixed() {
                    let t = self.onepoint.table("D_801231B4");
                    let z = if !self.save.adult { 100.0 } else { 120.0 };
                    self.onepoint.kf(t, 0).eye_target_init.z = z;
                    self.onepoint.kf(t, 1).eye_target_init.z = z;
                    let state1 = self.player.and_then(|h| self.actors.get(h)).and_then(|p| p.as_player()).map(|p| p.state_flags1()).unwrap_or(0);
                    if state1 & PLAYER_STATE1_27 != 0 {
                        self.onepoint.kf(t, 2).at_target_init.z = 0.0;
                    }
                    let Some((pos, shape_yaw)) = player.and_then(|h| self.actors.actor(h)).map(|a| (a.world_pos, a.shape_rot.y)) else { return 0 };
                    let mut sp_d0 = diff_to_sph_geo(pos, main.at);
                    sp_d0.yaw = sp_d0.yaw.wrapping_sub(shape_yaw);
                    self.onepoint.kf(t, 3).at_target_init = sph_geo_to_vec3(sp_d0);
                    let mut sp_d0 = diff_to_sph_geo(pos, main.eye);
                    sp_d0.yaw = sp_d0.yaw.wrapping_sub(shape_yaw);
                    let k = self.onepoint.kf(t, 3);
                    k.eye_target_init = sph_geo_to_vec3(sp_d0);
                    k.fov_target_init = main.fov;
                    k.timer_init = timer - 50;
                    set_cs_info(self, "D_801231B4", 4);
                } else {
                    let t = self.onepoint.table("D_80123254");
                    self.onepoint.kf(t, 1).timer_init = timer - 1;
                    self.onepoint.kf(t, 0).fov_target_init = main.fov;
                    for i in 0..2 {
                        let k = self.onepoint.kf(t, i);
                        k.at_target_init = main.at;
                        k.eye_target_init = main.eye;
                    }
                    set_cs_info(self, "D_80123254", 2);
                }
                self.func_800c0808(sub, player, CAM_SET_CS_C);
            }
            1000 => {
                let t = self.onepoint.table("D_801232A4");
                let k = self.onepoint.kf(t, 0);
                k.at_target_init = view.at;
                k.eye_target_init = view.eye;
                self.func_800c0808(sub, player, CAM_SET_CS_C);
            }
            5110 => {
                let t = self.onepoint.table("D_801239D4");
                self.onepoint.kf(t, 1).timer_init = 10;
                set_cs_info(self, "D_801239D4", 3);
                // func_800C0808 with the actor as Player.
                self.func_800c0808(sub, actor.map(|a| a.handle), CAM_SET_CS_C);
            }
            5120 => {
                self.func_8002df54(None, 8);
                set_cs_info(self, "D_80121314", 1);
                self.func_800c0808(sub, player, CAM_SET_CS_C);
            }
            _ => log::warn!("onepointdemo camera: demo number not found !! ({cs_id}): not ported"),
        }
        0
    }

    /// Player's `PlayerView`, or the actor's given for him (`func_800C0808`'s `Player*`).
    fn player_view_of(&self, h: ActorHandle) -> Option<crate::camera::PlayerView> {
        if Some(h) == self.player {
            return self.player_view();
        }
        let a = self.actors.actor(h)?;
        Some(crate::camera::PlayerView {
            pos: a.world_pos,
            shape_yaw: a.shape_rot.y,
            shape_pitch: a.shape_rot.x,
            world_yaw: a.world_rot.y,
            adult: self.save.adult,
            run_speed_limit: self.data.regs[if self.save.adult { 0 } else { 1 }].reg(45),
            gravity: a.gravity,
            climbing: false,
            state1: 0,
            iron_boots: false,
        })
    }
}
