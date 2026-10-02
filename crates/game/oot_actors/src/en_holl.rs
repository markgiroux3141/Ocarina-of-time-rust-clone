//! `En_Holl` (`ovl_En_Holl/z_en_holl.c`): the invisible planes between rooms. A transition
//! actor: it spawns from the scene's transition-actor list with its list index in the params'
//! top bits, and loads the room on the side Player is heading into.
//!
//! Params: bits 6..8 the kind (`sActionFuncs`), bits 0..5 a switch flag (kind 3).
//! - 0: a horizontal plane that fades as Player passes (`func_80A58DD4`), the only kind drawn;
//! - 1, 5: vertical planes (Player falls or climbs through; `func_80A591C0`, `func_80A593A4`);
//! - 2: a vertical plane without the dimming (`func_80A59520`);
//! - 3: a horizontal plane that only works once its switch flag is set (`func_80A59618`);
//! - 4, 6: horizontal planes 200 / 100 wide (`func_80A59014`), Kokiri Forest's.
//!
//! The plane mesh (`sPlaneDL`, in the overlay) isn't in the asset pack, so kind 0 is not
//! drawn; its `planeAlpha` is still computed.

use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_4, Actor};
use oot_game::actor_ctx::{ACTORCAT_DOOR, ActorImpl, ActorProfile};
use oot_game::play::PlayState;
use oot_game::scene::TRANSITION_ACTOR_PARAMS_INDEX_SHIFT;

pub const ACTOR_EN_HOLL: i16 = 0x0023;

/// `En_Holl_InitVars`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_EN_HOLL, name: "En_Holl", category: ACTORCAT_DOOR, flags: ACTOR_FLAG_4, object: "gameplay_keep" };

/// `PLANE_Y_MIN`, `PLANE_Y_MAX`, `PLANE_HALFWIDTH`, `PLANE_HALFWIDTH_2`.
const PLANE_Y_MIN: f32 = -50.0;
const PLANE_Y_MAX: f32 = 200.0;
const PLANE_HALFWIDTH: f32 = 100.0;
const PLANE_HALFWIDTH_2: f32 = 200.0;

/// `sHorizTriggerDists`: [0] load this side, [1] load the other side, [2]..[3] the fade.
const HORIZ_TRIGGER_DISTS: [[f32; 4]; 2] = [[200.0, 150.0, 100.0, 50.0], [100.0, 75.0, 50.0, 25.0]];

/// `SCENE_JYASINZOU` (the Spirit Temple uses the short distances).
const SCENE_JYASINZOU: u16 = 0x06;
/// `ENTR_SPOT04_0` (`EnHoll_IsKokiriLayer8`).
const ENTR_SPOT04_0: u16 = 0x00EE;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    /// `func_80A58DD4`.
    Horizontal,
    /// `func_80A591C0`.
    VerticalDown,
    /// `func_80A59520`.
    VerticalBg,
    /// `func_80A59618`.
    HorizontalSwitch,
    /// `func_80A59014`.
    HorizontalSimple,
    /// `func_80A593A4`.
    Vertical,
    /// `EnHoll_NextAction`.
    Next,
}

pub struct EnHoll {
    pub actor: Actor,
    pub plane_alpha: i16,
    pub side: u8,
    pub unk_14f: u8,
    action: Action,
}

impl EnHoll {
    /// `EnHoll_Init` (the init chain only sets the cull zone, which isn't ported).
    pub fn init(actor: Actor, _play: &mut PlayState) -> Box<dyn ActorImpl> {
        let mut h = EnHoll { actor, plane_alpha: 0, side: 0, unk_14f: 0, action: Action::Horizontal };
        h.choose_action();
        Box::new(h)
    }

    fn kind(&self) -> u16 {
        (self.actor.params as u16 >> 6) & 7
    }

    /// `GET_TRANSITION_ACTOR_INDEX`.
    fn transition_index(&self) -> usize {
        (self.actor.params as u16 >> TRANSITION_ACTOR_PARAMS_INDEX_SHIFT) as usize
    }

    /// `EnHoll_ChooseAction`.
    fn choose_action(&mut self) {
        self.action = match self.kind() {
            0 => Action::Horizontal,
            1 => Action::VerticalDown,
            2 => Action::VerticalBg,
            3 => Action::HorizontalSwitch,
            4 | 6 => Action::HorizontalSimple,
            _ => Action::Vertical,
        };
        if self.kind() == 0 {
            self.plane_alpha = 255;
        }
    }

    /// `func_8002DBD0`: `pos` in the actor's frame (x across, z through the plane).
    fn local(&self, pos: Vec3) -> Vec3 {
        let (c, s) = (eng_math::cos_s(self.actor.shape_rot.y), eng_math::sin_s(self.actor.shape_rot.y));
        let d = pos - self.actor.world_pos;
        Vec3::new(d.x * c - d.z * s, d.y, d.x * s + d.z * c)
    }

    fn side_room(play: &PlayState, idx: usize, side: usize) -> i8 {
        play.transi_actors.get(idx).map(|t| t.sides[side].0).unwrap_or(-1)
    }

    /// `EnHoll_IsKokiriLayer8`.
    fn is_kokiri_layer8(play: &PlayState) -> bool {
        play.save.entrance_index == ENTR_SPOT04_0 && play.save.scene_layer == 8
    }

    fn player_pos(play: &PlayState) -> Option<Vec3> {
        play.player.and_then(|h| play.actors.actor(h)).map(|a| a.world_pos)
    }

    /// `func_80A58DD4`.
    fn horizontal(&mut self, play: &mut PlayState) {
        let Some(pp) = Self::player_pos(play) else { return };
        let k = usize::from(play.scene_id == SCENE_JYASINZOU);
        let v = self.local(pp);
        self.side = if v.z < 0.0 { 0 } else { 1 };
        let abs_z = v.z.abs();
        if v.y > PLANE_Y_MIN && v.y < PLANE_Y_MAX && v.x.abs() < PLANE_HALFWIDTH && abs_z < HORIZ_TRIGGER_DISTS[k][0] {
            let idx = self.transition_index();
            if abs_z > HORIZ_TRIGGER_DISTS[k][1] {
                if play.room_ctx.prev.num >= 0 && play.room_ctx.status == 0 {
                    self.actor.room = Self::side_room(play, idx, self.side as usize);
                    play.room_ctx.swap();
                    play.room_change_done();
                }
            } else {
                self.actor.room = Self::side_room(play, idx, self.side as usize ^ 1);
                if play.room_ctx.prev.num < 0 {
                    play.room_request(self.actor.room);
                } else {
                    let d = &HORIZ_TRIGGER_DISTS[k];
                    let a = (255.0 / (d[2] - d[3])) * (abs_z - d[3]);
                    self.plane_alpha = (a as i16).clamp(0, 255);
                    if play.room_ctx.cur.num != self.actor.room {
                        play.room_ctx.swap();
                    }
                }
            }
        }
    }

    /// `func_80A59014`: Player's position, or in a cutscene the view's eye (`useViewEye`: the
    /// camera flying through loads the rooms it sees; the debug camera isn't ported).
    fn horizontal_simple(&mut self, play: &mut PlayState) {
        let use_view_eye = play.cs_ctx.state != oot_game::cutscene::CS_STATE_IDLE;
        let Some(pp) = (if use_view_eye { Some(play.view.eye) } else { Self::player_pos(play) }) else { return };
        let v = self.local(pp);
        let half = if self.kind() == 6 { PLANE_HALFWIDTH } else { PLANE_HALFWIDTH_2 };
        let kokiri8 = Self::is_kokiri_layer8(play);
        let abs_z = v.z.abs();
        if kokiri8 || (PLANE_Y_MIN < v.y && v.y < PLANE_Y_MAX && v.x.abs() < half && 100.0 > abs_z && abs_z > 50.0) {
            let side = if v.z < 0.0 { 0 } else { 1 };
            self.actor.room = Self::side_room(play, self.transition_index(), side);
            if self.actor.room != play.room_ctx.cur.num && play.room_request(self.actor.room) {
                self.action = Action::Next;
            }
        }
    }

    /// `func_80A591C0`.
    fn vertical_down(&mut self, play: &mut PlayState) {
        let abs_y = self.actor.y_dist_to_player.abs();
        if self.actor.xz_dist_to_player < 500.0 && abs_y < 700.0 {
            let idx = self.transition_index();
            play.unk_11e18 = if abs_y < 95.0 {
                0xFF
            } else if abs_y > 605.0 {
                0
            } else {
                (((605.0 - abs_y) as i16) as f32 * 0.5) as i16
            };
            if abs_y < 95.0 {
                self.actor.room = Self::side_room(play, idx, 1);
                let (tx, tz) = (self.actor.world_pos.x, self.actor.world_pos.z);
                if let Some(p) = play.player.and_then(|h| play.actors.actor_mut(h)) {
                    eng_math::smooth_step_to_f(&mut p.world_pos.x, tx, 1.0, 50.0, 10.0);
                    eng_math::smooth_step_to_f(&mut p.world_pos.z, tz, 1.0, 50.0, 10.0);
                }
                if self.actor.room != play.room_ctx.cur.num && play.room_request(self.actor.room) {
                    self.action = Action::Next;
                    self.unk_14f = 1;
                    if let Some(p) = play.player.and_then(|h| play.actors.actor_mut(h)) {
                        p.speed_xz = 0.0;
                    }
                }
            }
        } else if self.unk_14f != 0 {
            play.unk_11e18 = 0;
            self.unk_14f = 0;
        }
    }

    /// `func_80A593A4`.
    fn vertical(&mut self, play: &mut PlayState) {
        let abs_y = self.actor.y_dist_to_player.abs();
        if self.actor.xz_dist_to_player < 120.0 && abs_y < 200.0 {
            play.unk_11e18 = if abs_y < 50.0 { 0xFF } else { ((200.0 - abs_y) * 1.7) as i16 };
            if abs_y > 50.0 {
                let side = if 0.0 < self.actor.y_dist_to_player { 0 } else { 1 };
                self.actor.room = Self::side_room(play, self.transition_index(), side);
                if self.actor.room != play.room_ctx.cur.num && play.room_request(self.actor.room) {
                    self.action = Action::Next;
                    self.unk_14f = 1;
                }
            }
        } else if self.unk_14f != 0 {
            self.unk_14f = 0;
            play.unk_11e18 = 0;
        }
    }

    /// `func_80A59520`.
    fn vertical_bg(&mut self, play: &mut PlayState) {
        if self.actor.xz_dist_to_player < 120.0 {
            let abs_y = self.actor.y_dist_to_player.abs();
            if abs_y < 200.0 && abs_y > 50.0 {
                let side = if 0.0 < self.actor.y_dist_to_player { 0 } else { 1 };
                self.actor.room = Self::side_room(play, self.transition_index(), side);
                if self.actor.room != play.room_ctx.cur.num && play.room_request(self.actor.room) {
                    self.action = Action::Next;
                }
            }
        }
    }

    /// `func_80A59618`.
    fn horizontal_switch(&mut self, play: &mut PlayState) {
        if !play.flags.get_switch((self.actor.params & 0x3F) as i32) {
            if self.unk_14f != 0 {
                play.unk_11e18 = 0;
                self.unk_14f = 0;
            }
            return;
        }
        let Some(pp) = Self::player_pos(play) else { return };
        let v = self.local(pp);
        let abs_z = v.z.abs();
        if PLANE_Y_MIN < v.y && v.y < PLANE_Y_MAX && v.x.abs() < PLANE_HALFWIDTH_2 && abs_z < 100.0 {
            self.unk_14f = 1;
            play.unk_11e18 = (0xFF - ((abs_z - 50.0) * 5.9) as i32).clamp(0, 0xFF) as i16;
            if abs_z < 50.0 {
                let side = if v.z < 0.0 { 0 } else { 1 };
                self.actor.room = Self::side_room(play, self.transition_index(), side);
                if self.actor.room != play.room_ctx.cur.num && play.room_request(self.actor.room) {
                    self.action = Action::Next;
                }
            }
        } else if self.unk_14f != 0 {
            play.unk_11e18 = 0;
            self.unk_14f = 0;
        }
    }

    /// `EnHoll_NextAction`: once the room is in, the old one goes.
    fn next(&mut self, play: &mut PlayState) {
        if !Self::is_kokiri_layer8(play) && play.room_ctx.status == 0 {
            play.room_change_done();
            if play.unk_11e18 == 0 {
                self.unk_14f = 0;
            }
            self.choose_action();
        }
    }
}

impl ActorImpl for EnHoll {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `EnHoll_Update`.
    fn update(&mut self, play: &mut PlayState) {
        match self.action {
            Action::Horizontal => self.horizontal(play),
            Action::VerticalDown => self.vertical_down(play),
            Action::VerticalBg => self.vertical_bg(play),
            Action::HorizontalSwitch => self.horizontal_switch(play),
            Action::HorizontalSimple => self.horizontal_simple(play),
            Action::Vertical => self.vertical(play),
            Action::Next => self.next(play),
        }
    }
    /// `EnHoll_Destroy`: the transition-actor entry can spawn again.
    fn destroy(&mut self, play: &mut PlayState) {
        let i = self.transition_index();
        if let Some(t) = play.transi_actors.get_mut(i) {
            t.id = -t.id;
        }
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
