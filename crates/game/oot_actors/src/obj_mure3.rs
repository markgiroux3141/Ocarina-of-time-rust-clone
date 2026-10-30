//! `Obj_Mure3` (`ovl_Obj_Mure3/z_obj_mure3.c`): a set of rupees ("Tower of Rupees") spawned when
//! the camera comes near and taken away when it leaves (GAME-13 milestone 2). Kokiri Forest's
//! adult layers place one by the bean's lift, Death Mountain Crater's two layers and four grotto
//! layers others.
//!
//! - **Its sets** (`spawnFuncs`, by params bits 13 to 15): 0 (`func_80B9A9D0`) a tower of five
//!   blue rupees, 20 apart upwards; 1 (`func_80B9AA90`) a row of five green rupees, 20 apart across
//!   its facing, the middle one on it; 2 (`func_80B9ABA0`) a ring of six green rupees 40 round it
//!   (from its facing, a sixth of a turn apart) and a red one in the middle. Each is
//!   `Item_DropCollectible2` with 0x4000: hanging where it is, no gravity, in its room.
//! - **Waiting** (`func_80B9AF64`): once its `projectedPos` is within 1,150 across the view's x and
//!   z, it spawns its set (but those gone for good) and stays awake
//!   (`ACTOR_FLAG_UPDATE_CULLING_DISABLED`).
//! - **Out** (`func_80B9AFFC`, `func_80B9ADCC`): a rupee taken (its update gone) is gone for good;
//!   one with a parent (`Actor_HasParent`: carried off) sets its switch flag. Past 1,450 it takes
//!   the rest back (`func_80B9ACE4`: a rupee with a parent, or gone, stays gone) and waits again.
//!
//! The whole overlay is ported. No draw.
//!
//! Params: `switchFlag` bits 0 to 5 (set at init, it's gone; set when a rupee is carried off;
//! every placement has 0x3F, a temporary switch: the set's back on the next visit); bits 6 to 12
//! unused (placed 0x03); `type` bits 13 to 15 (0 the tower of blue rupees, 1 the row of green ones, 2 the ring of green
//! ones round a red; 3 and up read past the C's tables).
//! Flags: reads switch flag `switchFlag` at init (set: gone) and sets it when a rupee has a parent.

use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor};
use oot_game::actor_ctx::{ACTORCAT_BG, ActorHandle, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, Field, Flag, FlagUse, Kind};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};

use crate::en_item00::{ITEM00_RUPEE_BLUE, ITEM00_RUPEE_GREEN, ITEM00_RUPEE_RED, item_drop_collectible2};

/// `ACTOR_OBJ_MURE3` (`actor_table.h`: 0x01AB).
pub const ACTOR_OBJ_MURE3: i16 = 0x01AB;

/// `Obj_Mure3_Profile`: `ACTORCAT_BG`, no flags, `OBJECT_GAMEPLAY_KEEP`, no draw.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_OBJ_MURE3, name: "Obj_Mure3", category: ACTORCAT_BG, flags: 0, object: "gameplay_keep", schema: &SCHEMA };

/// `PARAMS_GET_U(params, 0, 6)`: the switch flag.
pub const SWITCH_FLAG: Field = Field::params("switchFlag", 0, 6, Kind::SwitchFlag, "set at init, it's gone; set when a rupee is carried off");
/// `PARAMS_GET_U(params, 13, 3)`: which set of rupees.
pub const TYPE: Field = Field::params(
    "type",
    13,
    3,
    Kind::Named(&[(0, "a tower of five blue rupees"), (1, "a row of five green rupees"), (2, "a ring of six green rupees round a red one")]),
    "which set of rupees it spawns (3 and up read past the C's tables)",
);

/// Bits 6 to 12: the data sets bits 6 and 7 (0x40FF); nothing reads them.
pub const BITS_6_12: Field = Field::params("params bits 6 to 12", 6, 7, Kind::Unused, "placed 0x03 (0x40FF); read by nothing");

/// Obj_Mure3's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema {
    fields: &[SWITCH_FLAG, BITS_6_12, TYPE],
    flags: &[FlagUse::new(Flag::Switch("switchFlag")).reads().sets().note("at init it's gone; set when a rupee is carried off (Actor_HasParent)")],
    todo: false,
    objects: &[],
    texts: &[],
};

/// `sRupeeCounts`: how many rupees each set has (its fourth entry, 0, for type 3).
const RUPEE_COUNTS: [usize; 4] = [5, 5, 7, 0];
/// `func_80B9AF64`'s and `func_80B9AFFC`'s squared distances: spawn within 1,150, take back past
/// 1,450.
const SPAWN_DIST_SQ: f32 = 1150.0 * 1150.0;
const DESPAWN_DIST_SQ: f32 = 1450.0 * 1450.0;

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// `func_80B9AF34`.
    Start,
    /// `func_80B9AF64`: waiting for the camera to come near.
    WaitNear,
    /// `func_80B9AFFC`: the set out, waiting for the camera to leave.
    WaitFar,
}

pub struct ObjMure3 {
    pub actor: Actor,
    pub action: Action,
    /// `unk_150`: the rupees out.
    pub rupees: [Option<ActorHandle>; 7],
    /// `unk_16C`: the rupees gone for good, by bit.
    pub unk_16c: u16,
}

impl ObjMure3 {
    /// `ObjMure3_Init`: gone with its switch flag set; else its culling volume (`sInitChain`:
    /// 100, 1800, 100), waiting (`func_80B9AF24`).
    pub fn init(mut actor: Actor, play: &mut PlayState) -> Box<dyn ActorImpl> {
        if play.flags.get_switch(SWITCH_FLAG.get(actor.params) as i32) {
            actor.kill();
        } else {
            actor.culling_volume_distance = 100.0;
            actor.culling_volume_scale = 1800.0;
            actor.culling_volume_downward = 100.0;
        }
        Box::new(ObjMure3 { actor, action: Action::Start, rupees: [None; 7], unk_16c: 0 })
    }

    fn set_type(&self) -> usize {
        TYPE.get(self.actor.params) as usize
    }

    fn gone(&self, i: usize) -> bool {
        (self.unk_16c >> i) & 1 != 0
    }

    /// One rupee (`0x4000 | item`) at `pos`, in its room, kept in slot `i`.
    fn drop(&mut self, play: &mut PlayState, i: usize, pos: Vec3, item: i16) {
        self.rupees[i] = item_drop_collectible2(play, pos, 0x4000 | item);
        if let Some(h) = self.rupees[i]
            && let Some(a) = play.actors.actor_mut(h)
        {
            a.room = self.actor.room;
        }
    }

    /// `func_80B9A9D0`: the tower, five blue rupees 20 apart upwards.
    fn func_80b9a9d0(&mut self, play: &mut PlayState) {
        let mut pos = self.actor.world_pos;
        for i in 0..5 {
            if !self.gone(i) {
                self.drop(play, i, pos, ITEM00_RUPEE_BLUE);
            }
            pos.y += 20.0;
        }
    }

    /// `func_80B9AA90`: the row, five green rupees 20 apart along its facing from -40.
    fn func_80b9aa90(&mut self, play: &mut PlayState) {
        let sn = eng_math::sin_s(self.actor.world_rot.y);
        let cos = eng_math::cos_s(self.actor.world_rot.y);
        let mut radius = -40.0;
        for i in 0..5 {
            if !self.gone(i) {
                let p = self.actor.world_pos;
                self.drop(play, i, Vec3::new(p.x + sn * radius, p.y, p.z + cos * radius), ITEM00_RUPEE_GREEN);
            }
            radius += 20.0;
        }
    }

    /// `func_80B9ABA0`: the ring, six green rupees 40 round it (0x2AAA apart from its facing), and
    /// a red one in the middle.
    fn func_80b9aba0(&mut self, play: &mut PlayState) {
        let p = self.actor.world_pos;
        let mut y_rot = self.actor.world_rot.y;
        for i in 0..6 {
            if !self.gone(i) {
                let pos = Vec3::new(eng_math::sin_s(y_rot) * 40.0 + p.x, p.y, eng_math::cos_s(y_rot) * 40.0 + p.z);
                self.drop(play, i, pos, ITEM00_RUPEE_GREEN);
            }
            y_rot = y_rot.wrapping_add(0x2AAA);
        }
        if !self.gone(6) {
            self.drop(play, 6, p, ITEM00_RUPEE_RED);
        }
    }

    /// `func_80B9ACE4`: the rupees still out taken back (one with a parent, or gone, stays gone).
    fn func_80b9ace4(&mut self, play: &mut PlayState) {
        let count = RUPEE_COUNTS.get(self.set_type()).copied().unwrap_or(0);
        for i in 0..count {
            if !self.gone(i)
                && let Some(h) = self.rupees[i]
            {
                match play.actors.actor_mut(h) {
                    Some(a) if !a.killed && !oot_game::get_item::actor_has_parent(a) => a.kill(),
                    _ => self.unk_16c |= 1 << i,
                }
            }
            self.rupees[i] = None;
        }
    }

    /// `func_80B9ADCC`: a rupee carried off sets the switch flag; one taken (its update gone) is
    /// gone for good.
    fn func_80b9adcc(&mut self, play: &mut PlayState) {
        let count = RUPEE_COUNTS.get(self.set_type()).copied().unwrap_or(0);
        for i in 0..count {
            let Some(h) = self.rupees[i] else { continue };
            if self.gone(i) {
                continue;
            }
            let a = play.actors.actor(h);
            if a.is_some_and(oot_game::get_item::actor_has_parent) {
                play.flags.set_switch(SWITCH_FLAG.get(self.actor.params) as i32);
            }
            if a.is_none_or(|a| a.killed) {
                self.unk_16c |= 1 << i;
                self.rupees[i] = None;
            }
        }
    }

    /// `Math3D_Dist1DSq(projectedPos.x, projectedPos.z)`.
    fn dist_sq(&self) -> f32 {
        let p = self.actor.projected_pos;
        p.x * p.x + p.z * p.z
    }
}

impl ActorImpl for ObjMure3 {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }

    /// `ObjMure3_Update`: `actionFunc`.
    fn update(&mut self, play: &mut PlayState) {
        match self.action {
            // func_80B9AF34: func_80B9AF54.
            Action::Start => self.action = Action::WaitNear,
            Action::WaitNear => {
                if self.dist_sq() < SPAWN_DIST_SQ {
                    self.actor.flags |= ACTOR_FLAG_UPDATE_CULLING_DISABLED;
                    match self.set_type() {
                        0 => self.func_80b9a9d0(play),
                        1 => self.func_80b9aa90(play),
                        2 => self.func_80b9aba0(play),
                        // @bug (game): `spawnFuncs` has three entries; types 3 to 7 call past it.
                        // None is placed.
                        t => log::warn!("Obj_Mure3: type {t} reads past spawnFuncs"),
                    }
                    // func_80B9AFEC.
                    self.action = Action::WaitFar;
                }
            }
            Action::WaitFar => {
                self.func_80b9adcc(play);
                if self.dist_sq() >= DESPAWN_DIST_SQ {
                    self.actor.flags &= !ACTOR_FLAG_UPDATE_CULLING_DISABLED;
                    self.func_80b9ace4(play);
                    // func_80B9AF54.
                    self.action = Action::WaitNear;
                }
            }
        }
    }

    /// No draw.
    fn draw(&self, _rs: &RenderState, _play: &PlayState, _view: &ViewInfo, _out: &mut DrawOut) {}

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
