//! `En_Door` (`ovl_En_Door/z_en_door.c`): doors with handles. A transition actor (it spawns
//! from the scene's transition-actor list, with its index in the params' top bits); Player
//! opens it (`func_80839800`), which loads the room behind it or, for a scene-exit door, starts
//! the exit under it.
//!
//! Params: the type in bits 7..9 (`EnDoorType`), a double door in bit 6, a switch flag (locked
//! doors) or a text id - 0x200 (checkable doors) in bits 0..5.
//!
//! The door waits for its scene's object (`sDoorInfo`: the Fire and Water Temples', the Shadow
//! Temple's, else `gameplay_keep`, or `gameplay_field_keep`'s door where that's loaded), then
//! idles: with Player within 20 across and 50 through it, and facing it within 0x3000, it
//! tells Player it can be opened (`doorType`, `doorDirection`, `doorActor`). Player sets
//! `playerIsOpening` and `openAnim`, and the door plays the opening animation for its side
//! and Link's age.
//!
//! Drawn as `EnDoor_Draw` draws it: `gDoorSkel` with limb 4 replaced by the side facing the
//! camera (or, while Player is still in the room it spawned in, the side for that room), from
//! meshes baked per door list and side (docs/adr/0012-actor-bakes.md).
//!
//! A checkable door (a shop closed at night, a locked house) has text instead
//! (`EnDoor_WaitForCheck` offers to talk within 40; `EnDoor_Check` waits for the box to close).
//!
//! Not ported: small keys (a locked door's lock never opens: there's no key count),
//! the lock's chains (`Actor_DrawDoorLock`), and the bubbles of a door opened underwater
//! (`EffectSsBubble`, with their count's `Rand_ZeroOne`).

use eng_gfx::{DrawCmd, MeshKey};
use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_0, ACTOR_FLAG_3, ACTOR_FLAG_4, ACTOR_FLAG_27, Actor};
use oot_game::actor_ctx::{ACTORCAT_DOOR, ActorImpl, ActorProfile, audio_play_actor_sfx2};
use oot_game::audio::sfx::{NA_SE_EV_CHAIN_KEY_UNLOCK, NA_SE_EV_DOOR_CLOSE, NA_SE_EV_IRON_DOOR_CLOSE, NA_SE_EV_IRON_DOOR_OPEN, NA_SE_OC_DOOR_OPEN};
use oot_game::pack::{BakeBody, LimbOverride, MeshBake, keys};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};
use oot_game::scene::TRANSITION_ACTOR_PARAMS_INDEX_SHIFT;
use oot_game::skelanime_std::{Anim, SkelAnimeStd};

use crate::player::{PLAYER_DOORTYPE_AJAR, PLAYER_DOORTYPE_HANDLE, Player, STATE1_27};

pub const ACTOR_EN_DOOR: i16 = 0x0009;

/// `En_Door_InitVars`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_DOOR, name: "En_Door", category: ACTORCAT_DOOR, flags: ACTOR_FLAG_4, object: "gameplay_keep" };

/// `DOOR_AJAR_SLAM_RANGE`, `DOOR_AJAR_OPEN_RANGE`.
const DOOR_AJAR_SLAM_RANGE: f32 = 120.0;
const DOOR_AJAR_OPEN_RANGE: f32 = 2.0 * DOOR_AJAR_SLAM_RANGE;

// `EnDoorType`.
pub const DOOR_ROOMLOAD: u16 = 0;
pub const DOOR_LOCKED: u16 = 1;
pub const DOOR_ROOMLOAD2: u16 = 2;
pub const DOOR_SCENEEXIT: u16 = 3;
pub const DOOR_AJAR: u16 = 4;
pub const DOOR_CHECKABLE: u16 = 5;
pub const DOOR_EVENING: u16 = 6;

/// `ENDOOR_PARAMS_TYPE_SHIFT`, `ENDOOR_PARAMS_TYPE_MASK`, `ENDOOR_PARAMS_DOUBLE_DOOR_FLAG`.
const TYPE_SHIFT: u32 = 7;
const TYPE_MASK: i16 = 7 << TYPE_SHIFT;
const DOUBLE_DOOR_FLAG: i16 = 0x40;

// `DOOR_OPEN_ANIM_*`.
pub const DOOR_OPEN_ANIM_ADULT_L: u8 = 0;
pub const DOOR_OPEN_ANIM_CHILD_L: u8 = 1;
pub const DOOR_OPEN_ANIM_ADULT_R: u8 = 2;
pub const DOOR_OPEN_ANIM_CHILD_R: u8 = 3;

// `SCENE_*` and `OBJECT_*`.
const SCENE_HIDAN: u16 = 0x04;
const SCENE_MIZUSIN: u16 = 0x05;
const SCENE_HAKADAN: u16 = 0x07;
const SCENE_HAKADANCH: u16 = 0x08;
const OBJECT_GAMEPLAY_KEEP: i16 = 0x0001;
const OBJECT_GAMEPLAY_FIELD_KEEP: i16 = 0x0002;
const OBJECT_HIDAN_OBJECTS: i16 = 0x002C;
const OBJECT_MIZU_OBJECTS: i16 = 0x0059;
const OBJECT_HAKA_DOOR: i16 = 0x0187;
/// `EVENTCHKINF_14`: Talon woken at Hyrule Castle.
const EVENTCHKINF_14: u16 = 0x14;

/// `EnDoorDListIndex`.
const DOOR_DL_DEFAULT: usize = 0;
const DOOR_DL_FIRE_TEMPLE: usize = 1;
const DOOR_DL_WATER_TEMPLE: usize = 2;
const DOOR_DL_SHADOW: usize = 3;
const DOOR_DL_DEFAULT_FIELD_KEEP: usize = 4;

/// `sDoorInfo`: scene, list index, object. The two keep entries stay last.
const DOOR_INFO: [(i32, usize, i16); 6] = [
    (SCENE_HIDAN as i32, DOOR_DL_FIRE_TEMPLE, OBJECT_HIDAN_OBJECTS),
    (SCENE_MIZUSIN as i32, DOOR_DL_WATER_TEMPLE, OBJECT_MIZU_OBJECTS),
    (SCENE_HAKADAN as i32, DOOR_DL_SHADOW, OBJECT_HAKA_DOOR),
    (SCENE_HAKADANCH as i32, DOOR_DL_SHADOW, OBJECT_HAKA_DOOR),
    (-1, DOOR_DL_DEFAULT, OBJECT_GAMEPLAY_KEEP),
    (-1, DOOR_DL_DEFAULT_FIELD_KEEP, OBJECT_GAMEPLAY_FIELD_KEEP),
];

/// `sDoorAnims`.
const DOOR_ANIMS: [&str; 4] = ["gDoorAdultOpeningLeftAnim", "gDoorChildOpeningLeftAnim", "gDoorAdultOpeningRightAnim", "gDoorChildOpeningRightAnim"];
/// `sDoorAnimOpenFrames`, `sDoorAnimCloseFrames` (where the sounds play).
const DOOR_ANIM_OPEN_FRAMES: [f32; 4] = [25.0, 25.0, 25.0, 25.0];
const DOOR_ANIM_CLOSE_FRAMES: [f32; 4] = [60.0, 70.0, 60.0, 70.0];

/// `sDoorDLists`: each list index's two sides, `(file, left, right)`.
const DOOR_DLISTS: [(&str, &str, &str); 5] = [
    ("gameplay_keep", "gDoorLeftDL", "gDoorRightDL"),
    ("object_hidan_objects", "gFireTempleDoorWithHandleLeftDL", "gFireTempleDoorWithHandleRightDL"),
    ("object_mizu_objects", "gWaterTempleDoorLeftDL", "gWaterTempleDoorRightDL"),
    ("object_haka_door", "gShadowDoorLeftDL", "gShadowDoorRightDL"),
    ("gameplay_field_keep", "gFieldDoorLeftDL", "gFieldDoorRightDL"),
];

/// The bake of list `index`'s side (0 left, 1 right): `gDoorSkel` with limb 4's list replaced.
fn bake_name(index: usize, side: usize) -> String {
    format!("En_Door/{index}/{}", if side == 0 { "left" } else { "right" })
}

/// The bakes of `gDoorLeftDL` / `gDoorRightDL` drawn alone over an ajar door.
fn ajar_bake_name(side: usize) -> String {
    format!("En_Door/ajar/{}", if side == 0 { "left" } else { "right" })
}

/// The meshes `EnDoor_Draw` draws.
pub fn bakes() -> Vec<MeshBake> {
    let mut v = Vec::new();
    for (i, (file, left, right)) in DOOR_DLISTS.iter().enumerate() {
        for (side, sym) in [left, right].into_iter().enumerate() {
            v.push(MeshBake {
                name: bake_name(i, side),
                object: "gameplay_keep".into(),
                segments: Vec::new(),
                prelude: Vec::new(),
                // EnDoor_OverrideLimbDraw, limbIndex 4.
                body: BakeBody::Skeleton { file: "gameplay_keep".into(), symbol: "gDoorSkel".into(), limbs: vec![LimbOverride { limb: 3, file: (*file).into(), symbol: (*sym).into() }] },
            });
        }
    }
    for (side, sym) in ["gDoorLeftDL", "gDoorRightDL"].into_iter().enumerate() {
        v.push(MeshBake { name: ajar_bake_name(side), object: "gameplay_keep".into(), segments: Vec::new(), prelude: Vec::new(), body: BakeBody::DLists(vec![("gameplay_keep".into(), sym.into())]) });
    }
    v
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `EnDoor_SetupType`: waiting for the object.
    SetupType,
    /// `EnDoor_Idle`.
    Idle,
    /// `EnDoor_WaitForCheck`.
    WaitForCheck,
    /// `EnDoor_Check`.
    Check,
    /// `EnDoor_AjarWait`.
    AjarWait,
    /// `EnDoor_AjarOpen`.
    AjarOpen,
    /// `EnDoor_AjarClose`.
    AjarClose,
    /// `EnDoor_Open`.
    Open,
}

pub struct EnDoor {
    pub actor: Actor,
    /// `DOOR_ACTOR_BASE`: `skelAnime`, `openAnim`, `playerIsOpening`.
    pub skel: SkelAnimeStd,
    pub open_anim: u8,
    pub player_is_opening: bool,
    /// `unk_192`: 1 for the second half of a double door.
    pub unk_192: u8,
    pub required_obj_bank_index: Option<usize>,
    pub dlist_index: usize,
    pub lock_timer: i16,
    pub action: Action,
    anims: Vec<Anim>,
}

impl EnDoor {
    /// `EnDoor_Init`.
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        // sInitChain: targetMode 0 (uncullZoneForward 4000: culling isn't ported).
        actor.target_mode = 0;
        let anims: Vec<Anim> = match play.assets.clone() {
            Some(a) => DOOR_ANIMS.iter().filter_map(|n| a.animation("gameplay_keep", n).map_err(|e| log::error!("En_Door: {e:#}")).ok()).collect(),
            None => Vec::new(),
        };
        // SkelAnime_Init(&gDoorSkel, &gDoorAdultOpeningLeftAnim, 5): the skeleton's 4 limbs.
        let skel = SkelAnimeStd::init_flex(4, anims.first().cloned());
        let mut d = EnDoor { actor, skel, open_anim: 0, player_is_opening: false, unk_192: 0, required_obj_bank_index: None, dlist_index: 0, lock_timer: 0, action: Action::SetupType, anims };
        if d.anims.len() != DOOR_ANIMS.len() {
            d.actor.kill();
            return Box::new(d);
        }
        let mut i = DOOR_INFO[..4].iter().position(|e| e.0 == play.scene_id as i32).unwrap_or(4);
        if i >= 4 && play.object_ctx.get_index(OBJECT_GAMEPLAY_FIELD_KEEP).is_some() {
            i += 1;
        }
        let (_, dlist_index, object) = DOOR_INFO[i];
        d.dlist_index = dlist_index;
        let Some(bank) = play.object_ctx.get_index(object) else {
            d.actor.kill();
            return Box::new(d);
        };
        d.required_obj_bank_index = Some(bank);
        if d.actor.obj_bank_index == d.required_obj_bank_index {
            d.setup_type(play);
        } else {
            d.action = Action::SetupType;
        }
        // Double doors: the other half, 30 to the side, turned round.
        if d.actor.params & DOUBLE_DOOR_FLAG != 0 {
            let (c, s) = (eng_math::cos_s(d.actor.shape_rot.y), eng_math::sin_s(d.actor.shape_rot.y));
            let (x_offset, z_offset) = (c * 30.0, s * 30.0);
            let pos = Vec3::new(d.actor.world_pos.x + x_offset, d.actor.world_pos.y, d.actor.world_pos.z - z_offset);
            let rot = [0, d.actor.shape_rot.y.wrapping_add(-0x8000i32 as i16), 0];
            let params = d.actor.params & !DOUBLE_DOOR_FLAG;
            match play.actor_spawn_as_child(&mut d.actor, ACTOR_EN_DOOR, pos, rot, params) {
                Ok(h) => {
                    if let Some(other) = play.actors.downcast_mut::<EnDoor>(h) {
                        other.unk_192 = 1;
                    }
                }
                Err(e) => log::debug!("En_Door: the double door's other half: {e:?}"),
            }
            d.actor.world_pos.x -= x_offset;
            d.actor.world_pos.z += z_offset;
        }
        // Actor_SetFocus(70).
        d.actor.focus_pos = d.actor.world_pos + Vec3::Y * 70.0;
        Box::new(d)
    }

    /// `ENDOOR_GET_TYPE`.
    pub fn door_type(&self) -> u16 {
        (self.actor.params as u16 >> TYPE_SHIFT) & 7
    }

    /// `GET_TRANSITION_ACTOR_INDEX`.
    pub fn transition_index(&self) -> usize {
        (self.actor.params as u16 >> TRANSITION_ACTOR_PARAMS_INDEX_SHIFT) as usize
    }

    /// `EnDoor_SetupType`.
    fn setup_type(&mut self, play: &mut PlayState) {
        let Some(bank) = self.required_obj_bank_index.filter(|&b| play.object_ctx.is_loaded(b)) else { return };
        let mut door_type = self.door_type();
        self.actor.flags &= !ACTOR_FLAG_4;
        self.actor.obj_bank_index = Some(bank);
        self.action = Action::Idle;
        if door_type == DOOR_EVENING {
            let t = play.save.day_time;
            door_type = if t > oot_game::env::clock_time(18, 0) as u16 && t < oot_game::env::clock_time(21, 0) as u16 { DOOR_SCENEEXIT } else { DOOR_CHECKABLE };
        }
        self.actor.world_rot.y = 0;
        if door_type == DOOR_LOCKED {
            if !play.flags.get_switch((self.actor.params & 0x3F) as i32) {
                self.lock_timer = 10;
            }
        } else if door_type == DOOR_AJAR {
            let d = play.player.and_then(|h| play.actors.actor(h)).map(|p| (p.world_pos - self.actor.world_pos).truncate_y_len());
            if d.is_some_and(|d| d > DOOR_AJAR_SLAM_RANGE) {
                self.action = Action::AjarWait;
                self.actor.world_rot.y = -0x1800;
            }
        } else if door_type == DOOR_CHECKABLE {
            self.actor.text_id = (self.actor.params & 0x3F) as u16 + 0x0200;
            if self.actor.text_id == 0x0229 && !play.save.get_event_chk_inf(EVENTCHKINF_14) {
                // Talon's house door: openable at any time until Talon's woken at the castle.
                door_type = DOOR_SCENEEXIT;
            } else {
                self.action = Action::WaitForCheck;
                self.actor.flags |= ACTOR_FLAG_0 | ACTOR_FLAG_3 | ACTOR_FLAG_27;
            }
        }
        // The type it was loaded with gives way to the new one.
        self.actor.params = (self.actor.params & !TYPE_MASK) | ((door_type as i16) << TYPE_SHIFT);
    }

    /// `func_8002DBD0`: `pos` in the door's frame (x across, z through it).
    fn local(&self, pos: Vec3) -> Vec3 {
        let (c, s) = (eng_math::cos_s(self.actor.shape_rot.y), eng_math::sin_s(self.actor.shape_rot.y));
        let d = pos - self.actor.world_pos;
        Vec3::new(d.x * c - d.z * s, d.y, d.x * s + d.z * c)
    }

    /// `EnDoor_Idle`.
    fn idle(&mut self, play: &mut PlayState) {
        let door_type = self.door_type();
        let Some(ph) = play.player else { return };
        let Some((pp, p_yaw, swimming)) = play.actors.downcast::<Player>(ph).map(|p| (p.actor.world_pos, p.actor.shape_rot.y, p.state1 & STATE1_27 != 0)) else { return };
        let rel = self.local(pp);
        if self.player_is_opening {
            self.action = Action::Open;
            let anim = self.anims[self.open_anim as usize & 3].clone();
            // Animation_PlayOnceSetSpeed.
            let last = anim.last_frame();
            self.skel.change(anim, if swimming { 0.75 } else { 1.5 }, 0.0, last, oot_game::skelanime_std::ANIMMODE_ONCE, 0.0);
            if self.lock_timer != 0 {
                // dungeonKeys[mapIndex]--: no keys yet (Player never opens a locked door here).
                play.flags.set_switch((self.actor.params & 0x3F) as i32);
                audio_play_actor_sfx2(play, NA_SE_EV_CHAIN_KEY_UNLOCK);
            }
        } else if !play.player_in_cs_mode() {
            if rel.y.abs() < 20.0 && rel.x.abs() < 20.0 && rel.z.abs() < 50.0 {
                let mut yaw_diff = p_yaw.wrapping_sub(self.actor.shape_rot.y);
                if rel.z > 0.0 {
                    yaw_diff = (-0x8000i32 as i16).wrapping_sub(yaw_diff);
                }
                if (yaw_diff as i32).abs() < 0x3000 {
                    if self.lock_timer != 0 {
                        // dungeonKeys[mapIndex] <= 0 (there are no keys): Navi's text -0x203.
                        return;
                    }
                    let me = play.cur_actor;
                    if let Some(p) = play.actors.downcast_mut::<Player>(ph) {
                        p.door_type = if door_type == DOOR_AJAR { PLAYER_DOORTYPE_AJAR } else { PLAYER_DOORTYPE_HANDLE };
                        p.door_direction = if rel.z >= 0.0 { 1 } else { -1 };
                        p.door_actor = me;
                    }
                }
            } else if door_type == DOOR_AJAR && self.actor.xz_dist_to_player > DOOR_AJAR_OPEN_RANGE {
                self.action = Action::AjarOpen;
            }
        }
    }

    /// `EnDoor_Open`.
    fn open(&mut self, play: &mut PlayState) {
        let iron = play.scene_id == SCENE_HAKADAN || play.scene_id == SCENE_HAKADANCH || play.scene_id == SCENE_HIDAN;
        // DECR(lockTimer) == 0.
        if self.lock_timer != 0 {
            self.lock_timer -= 1;
        }
        if self.lock_timer == 0 {
            if self.skel.update() {
                self.action = Action::Idle;
                self.player_is_opening = false;
            } else if self.skel.on_frame(DOOR_ANIM_OPEN_FRAMES[self.open_anim as usize & 3]) {
                audio_play_actor_sfx2(play, if iron { NA_SE_EV_IRON_DOOR_OPEN } else { NA_SE_OC_DOOR_OPEN });
                // The bubbles when opened underwater (playSpeed < 1.5): not ported.
            } else if self.skel.on_frame(DOOR_ANIM_CLOSE_FRAMES[self.open_anim as usize & 3]) {
                audio_play_actor_sfx2(play, if iron { NA_SE_EV_IRON_DOOR_CLOSE } else { NA_SE_EV_DOOR_CLOSE });
            }
        }
    }
}

trait XzLen {
    fn truncate_y_len(self) -> f32;
}

impl XzLen for Vec3 {
    /// `Actor_WorldDistXZToActor`'s distance.
    fn truncate_y_len(self) -> f32 {
        (self.x * self.x + self.z * self.z).sqrt()
    }
}

/// Indices into the render state's extras.
mod rs {
    /// `angles`: `world.rot.y` (the ajar door's swing).
    pub const WORLD_ROT_Y: usize = 0;
}

impl ActorImpl for EnDoor {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `EnDoor_Update`.
    fn update(&mut self, play: &mut PlayState) {
        match self.action {
            Action::SetupType => self.setup_type(play),
            Action::Idle => self.idle(play),
            // EnDoor_WaitForCheck: DOOR_CHECK_RANGE 40.
            Action::WaitForCheck => {
                if oot_game::npc::process_talk_request(&mut self.actor) {
                    self.action = Action::Check;
                } else {
                    oot_game::npc::offer_talk(play, &self.actor, 40.0);
                }
            }
            // EnDoor_Check.
            Action::Check => {
                if oot_game::npc::textbox_is_closing(play) {
                    self.action = Action::WaitForCheck;
                }
            }
            Action::AjarWait => {
                if self.actor.xz_dist_to_player < DOOR_AJAR_SLAM_RANGE {
                    self.action = Action::AjarClose;
                }
            }
            Action::AjarOpen => {
                if self.actor.xz_dist_to_player < DOOR_AJAR_SLAM_RANGE {
                    self.action = Action::AjarClose;
                } else if eng_math::scaled_step_to_s(&mut self.actor.world_rot.y, -0x1800, 0x100) {
                    self.action = Action::AjarWait;
                }
            }
            Action::AjarClose => {
                if eng_math::scaled_step_to_s(&mut self.actor.world_rot.y, 0, 0x700) {
                    self.action = Action::Idle;
                }
            }
            Action::Open => self.open(play),
        }
    }
    /// `EnDoor_Destroy`: the transition-actor entry can spawn again.
    fn destroy(&mut self, play: &mut PlayState) {
        let i = self.transition_index();
        if let Some(t) = play.transi_actors.get_mut(i)
            && t.id < 0
        {
            t.id = -t.id;
        }
    }
    fn render_state(&self) -> RenderState {
        let mut rs = RenderState::of(&self.actor);
        rs.joints = Some(eng_anim::anim::JointTable { rot: self.skel.joint_table.clone(), face: 0 });
        rs.angles = vec![self.actor.world_rot.y];
        rs
    }
    /// `EnDoor_Draw`: the skeleton with the side `EnDoor_OverrideLimbDraw` picks for limb 4,
    /// then the ajar door's other face.
    fn draw(&self, rs: &RenderState, play: &PlayState, view: &ViewInfo, out: &mut DrawOut) {
        if self.actor.obj_bank_index.is_none() || self.actor.obj_bank_index != self.required_obj_bank_index {
            return;
        }
        let (Some(joints), Some(&world_rot_y)) = (&rs.joints, rs.angles.get(rs::WORLD_ROT_Y)) else { return };
        let Some(skeleton) = play.assets.as_ref().and_then(|a| a.skeleton("gameplay_keep", "gDoorSkel").ok()) else { return };
        let mut side = 0;
        let bones = skeleton.pose_override(&joints.rot, |limb, _pos, rot| {
            if limb == 4 {
                let t = play.transi_actors.get(self.transition_index());
                rot[2] = rot[2].wrapping_add(world_rot_y);
                let same_room = t.is_some_and(|t| t.sides[0].0 == t.sides[1].0);
                if play.room_ctx.prev.num >= 0 || same_room {
                    // The side facing the camera.
                    let to_door = eng_math::atan2_s(rs.pos.z - view.eye.z, rs.pos.x - view.eye.x);
                    let j3 = joints.rot.get(3).map(|r| r[2]).unwrap_or(0);
                    let rot_diff = rs.rot[1].wrapping_add(j3).wrapping_add(rot[2]).wrapping_sub(to_door);
                    side = if (rot_diff as i32).abs() < 0x4000 { 0 } else { 1 };
                } else {
                    let mut i = self.unk_192 as usize;
                    if t.is_some_and(|t| t.sides[0].0 != self.actor.room) {
                        i ^= 1;
                    }
                    side = i;
                }
            }
            glam::Mat4::IDENTITY
        });
        let m = oot_game::play::actor_draw_matrix(rs);
        out.opa.push(DrawCmd { mesh: MeshKey::named(keys::bake(&bake_name(self.dlist_index, side))), transform: m, bones, params: Default::default() });
        if world_rot_y != 0 {
            let s = if world_rot_y > 0 { 1 } else { 0 };
            out.opa.push(DrawCmd::new(MeshKey::named(keys::bake(&ajar_bake_name(s))), m));
        }
        // lockTimer != 0: Actor_DrawDoorLock (the chains) isn't ported.
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
