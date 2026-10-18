//! `Obj_Mure` (`ovl_Obj_Mure/z_obj_mure.c`): a group of fish, bugs or butterflies, spawned while
//! the camera is near. `params`: `>> 12` the count (0 for the pattern's), `(>> 8) & 7` the
//! pattern (`ptn`), `(>> 5) & 3` the spawn kind (`svNum`), `& 0x1F` the type.
//!
//! Within `sZClip` of the camera (`projectedPos.z`) it spawns its children
//! (`ObjMure_SpawnActors`), and kills those left beyond it (40 more); in between, each frame, it
//! steers them (`ObjMure_GroupBehavior0` for fish and bugs: some follow others, one follows Link
//! when he stays near; `ObjMure_GroupBehavior1` for butterflies), through each child's
//! `actor.child`. The first of a butterfly group is the one that turns into a fairy (params 1).
//!
//! Its children: `En_Butte` (ported), `En_Fish` and `En_Insect` (not ported: placeholders).
//!
//! Params: `type` bits 0 to 4 (`ObjMureType`: 2 fish, `En_Fish`; 3 bugs, `En_Insect`; 4
//! butterflies, `En_Butte`; 0 grass and 1 nothing get no culling and are killed, as is 5 and up);
//! `svNum` bits 5 and 6 (0: the dead stay dead when it respawns them, `ObjMure_SpawnActors0`; 1: all
//! respawned, `ObjMure_SpawnActors1`; 2, 3: none); `ptn` bits 8 to 10 (the pattern: its count when
//! `chNum` is 0, `sMaxChildSpawns` 12, 9, 8, 0; 4 and up killed); `chNum` bits 12 to 15 (the count;
//! 0 the pattern's).
//! Flags: none.

use glam::Vec3;
use oot_game::actor::{ACTOR_FLAG_UPDATE_CULLING_DISABLED, Actor};
use oot_game::actor_ctx::{ACTORCAT_ITEMACTION, ActorHandle, ActorImpl, ActorProfile};
use oot_game::actor_schema::{ActorSchema, Field, Kind};
use oot_game::play::{DrawOut, PlayState, RenderState, ViewInfo};

use crate::en_butte::ACTOR_EN_BUTTE;
use crate::en_kusa::{ACTOR_EN_KUSA, ACTOR_FLAG_GRASS_DESTROYED};

/// `ACTOR_OBJ_MURE` (`actor_table.h`: 0x0094).
pub const ACTOR_OBJ_MURE: i16 = 0x0094;
/// `ACTOR_EN_INSECT`, `ACTOR_EN_FISH` (`actor_table.h`: 0x0020, 0x0021).
const ACTOR_EN_INSECT: i16 = 0x0020;
const ACTOR_EN_FISH: i16 = 0x0021;

/// `Obj_Mure_Profile`.
pub const PROFILE: ActorProfile = ActorProfile { id: ACTOR_OBJ_MURE, name: "Obj_Mure", category: ACTORCAT_ITEMACTION, flags: 0, object: "gameplay_keep", schema: &SCHEMA };

/// `PARAMS_GET_U(thisx->params, 0, 5)`: what it spawns.
pub const TYPE: Field = Field::params(
    "type",
    0,
    5,
    Kind::Named(&[(0, "OBJMURE_TYPE_GRASS"), (1, "OBJMURE_TYPE_UNDEFINED"), (2, "OBJMURE_TYPE_FISH"), (3, "OBJMURE_TYPE_BUGS"), (4, "OBJMURE_TYPE_BUTTERFLY")]),
    "fish, bugs or butterflies (grass and 1 get no culling: killed)",
);
/// `PARAMS_GET_U(thisx->params, 5, 2)`: how it respawns them.
pub const SV_NUM: Field = Field::params(
    "svNum",
    5,
    2,
    Kind::Named(&[(0, "the dead stay dead (ObjMure_SpawnActors0)"), (1, "all respawned (ObjMure_SpawnActors1)"), (2, "none spawned"), (3, "none spawned")]),
    "how it spawns its group",
);
/// `PARAMS_GET_U(thisx->params, 8, 3)`: the pattern.
pub const PTN: Field = Field::params("ptn", 8, 3, Kind::Range(0, 3), "the pattern: its count when chNum is 0 (sMaxChildSpawns: 12, 9, 8, 0); 4 and up killed");
/// `PARAMS_GET_U(thisx->params, 12, 4)`: the count.
pub const CH_NUM: Field = Field::params("chNum", 12, 4, Kind::Range(0, 15), "how many (0: the pattern's)");

/// Obj_Mure's configuration (docs/adr/0066-the-actor-schema.md).
pub const SCHEMA: ActorSchema = ActorSchema { fields: &[TYPE, SV_NUM, PTN, CH_NUM], flags: &[], todo: false, objects: &[] };

/// `OBJMURE_MAX_SPAWNS`.
pub const OBJMURE_MAX_SPAWNS: usize = 15;

// `ObjMureType`.
const OBJMURE_TYPE_FISH: i16 = 2;
const OBJMURE_TYPE_BUGS: i16 = 3;
const OBJMURE_TYPE_BUTTERFLY: i16 = 4;

// `ObjMureChildState`.
const OBJMURE_CHILD_STATE_0: u8 = 0;
/// Dead.
const OBJMURE_CHILD_STATE_1: u8 = 1;
const OBJMURE_CHILD_STATE_2: u8 = 2;

/// `sZClip`: how near the camera the group comes alive, by type.
const Z_CLIP: [f32; 5] = [1600.0, 1600.0, 1000.0, 1000.0, 1000.0];
/// `sMaxChildSpawns`: the count by pattern when the params give none.
const MAX_CHILD_SPAWNS: [i32; 4] = [12, 9, 8, 0];
/// `sSpawnActorIds`.
const SPAWN_ACTOR_IDS: [i16; 5] = [ACTOR_EN_KUSA, 0, ACTOR_EN_FISH, ACTOR_EN_INSECT, ACTOR_EN_BUTTE];
/// `sSpawnParams`: `EN_FISH_TYPE_NORMAL` (-1), `INSECT_TYPE_PERMANENT` (0).
const SPAWN_PARAMS: [i16; 5] = [0, 2, -1, 0, -1];

/// `actionFunc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    InitialAction,
    CulledState,
    ActiveState,
}

pub struct ObjMure {
    pub actor: Actor,
    pub action: Action,
    pub ch_num: i16,
    pub ptn: i16,
    pub sv_num: i16,
    pub ty: i16,
    pub children: [Option<ActorHandle>; OBJMURE_MAX_SPAWNS],
    pub children_states: [u8; OBJMURE_MAX_SPAWNS],
    /// `unk_1A4`: frames to the next regrouping.
    pub unk_1a4: i16,
    /// `unk_1A6`: the group's phase (tight or loose).
    pub unk_1a6: bool,
    /// `unk_1A8`: frames Link has stayed within 120.
    pub unk_1a8: i16,
}

impl ObjMure {
    /// `ObjMure_Init`.
    pub fn init(actor: Actor, _play: &mut PlayState) -> Box<dyn ActorImpl> {
        // schema: printed
        let p = actor.params as u16;
        let mut this = ObjMure {
            ch_num: CH_NUM.get(actor.params),
            ptn: PTN.get(actor.params),
            sv_num: SV_NUM.get(actor.params),
            ty: TYPE.get(actor.params),
            action: Action::InitialAction,
            children: [None; OBJMURE_MAX_SPAWNS],
            children_states: [0; OBJMURE_MAX_SPAWNS],
            unk_1a4: 0,
            unk_1a6: false,
            unk_1a8: 0,
            actor,
        };
        if this.ptn >= 4 || this.ty >= 5 {
            log::warn!("Obj_Mure: error, swarm of enemies (arg_data {p:#06x})");
            this.actor.kill();
            return Box::new(this);
        }
        if !this.set_culling() {
            this.actor.kill();
            return Box::new(this);
        }
        log::debug!("Obj_Mure: swarm (arg_data {p:#06x})(chNum({}) ptn({}) svNum({}) type({}))", this.ch_num, this.ptn, this.sv_num, this.ty);
        if this.get_max_child_spawns() <= 0 {
            log::warn!("Obj_Mure: warning, the number of individuals is not set (arg_data {p:#06x})");
        }
        Box::new(this)
    }

    /// `ObjMure_SetCulling` (`ObjMure_SetCullingImpl`): fish, bugs and butterflies only.
    fn set_culling(&mut self) -> bool {
        match self.ty {
            OBJMURE_TYPE_FISH | OBJMURE_TYPE_BUGS | OBJMURE_TYPE_BUTTERFLY => {
                // sInitChain.
                self.actor.culling_volume_distance = 1200.0;
                self.actor.culling_volume_scale = 200.0;
                self.actor.culling_volume_downward = 1200.0;
                true
            }
            _ => {
                // schema: printed
                log::warn!("Obj_Mure: error, culling is not set (arg_data {:#06x})", self.actor.params);
                false
            }
        }
    }

    /// `ObjMure_GetMaxChildSpawns`.
    fn get_max_child_spawns(&self) -> i32 {
        if self.ch_num == 0 { MAX_CHILD_SPAWNS[self.ptn as usize] } else { self.ch_num as i32 }
    }

    /// `ObjMure_GetSpawnPos`: every child at the group's position.
    fn get_spawn_pos(&self) -> Vec3 {
        self.actor.world_pos
    }

    fn spawn_child(&mut self, play: &mut PlayState, params: i16) -> Option<ActorHandle> {
        let pos = self.get_spawn_pos();
        let r = self.actor.world_rot;
        let h = play.actor_spawn(SPAWN_ACTOR_IDS[self.ty as usize], pos, [r.x, r.y, r.z], params).ok();
        if let Some(a) = h.and_then(|h| play.actors.actor_mut(h)) {
            a.room = self.actor.room;
        }
        h
    }

    /// `ObjMure_SpawnActors0`: the children not dead again (those cut down marked so).
    fn spawn_actors0(&mut self, play: &mut PlayState) {
        for i in 0..self.get_max_child_spawns() as usize {
            if self.children[i].is_some() {
                // schema: printed
                log::warn!("Obj_Mure: error, I already have a child (arg_data {:#06x})", self.actor.params);
            }
            match self.children_states[i] {
                OBJMURE_CHILD_STATE_1 => {}
                OBJMURE_CHILD_STATE_2 => {
                    self.children[i] = self.spawn_child(play, SPAWN_PARAMS[self.ty as usize]);
                    match self.children[i].and_then(|h| play.actors.actor_mut(h)) {
                        Some(a) => a.flags |= ACTOR_FLAG_GRASS_DESTROYED,
                        None => log::warn!("Obj_Mure: warning, failed to spawn"),
                    }
                }
                _ => {
                    self.children[i] = self.spawn_child(play, SPAWN_PARAMS[self.ty as usize]);
                    if self.children[i].is_none() {
                        log::warn!("Obj_Mure: warning, failed to spawn");
                    }
                }
            }
        }
    }

    /// `ObjMure_SpawnActors1`: all the children, alive again; a butterfly group's first the
    /// fairy one.
    fn spawn_actors1(&mut self, play: &mut PlayState) {
        for i in 0..self.get_max_child_spawns() as usize {
            if self.children[i].is_some() {
                // schema: printed
                log::warn!("Obj_Mure: error, I already have a child (arg_data {:#06x})", self.actor.params);
            }
            let params = if self.ty == 4 && i == 0 { 1 } else { SPAWN_PARAMS[self.ty as usize] };
            self.children[i] = self.spawn_child(play, params);
            if self.children[i].is_some() {
                self.children_states[i] = OBJMURE_CHILD_STATE_0;
            } else {
                self.children_states[i] = OBJMURE_CHILD_STATE_1;
                log::warn!("Obj_Mure: warning, failed to spawn");
            }
        }
    }

    /// `ObjMure_SpawnActors`.
    fn spawn_actors(&mut self, play: &mut PlayState) {
        match self.sv_num {
            0 => self.spawn_actors0(play),
            1 => self.spawn_actors1(play),
            _ => {}
        }
    }

    /// `ObjMure_KillActors` (`ObjMure_KillActorsImpl`): all but those Player holds.
    fn kill_actors(&mut self, play: &mut PlayState) {
        for i in 0..self.get_max_child_spawns() as usize {
            match self.children_states[i] {
                OBJMURE_CHILD_STATE_1 => self.children[i] = None,
                OBJMURE_CHILD_STATE_2 => {
                    if let Some(h) = self.children[i].take()
                        && let Some(a) = play.actors.actor_mut(h)
                    {
                        a.kill();
                    }
                }
                _ => {
                    if let Some(h) = self.children[i].take()
                        && let Some(a) = play.actors.actor_mut(h)
                        && a.parent.is_none()
                    {
                        a.kill();
                    }
                }
            }
        }
    }

    /// Whether `h`'s actor is gone (`update == NULL`).
    fn gone(play: &PlayState, h: ActorHandle) -> bool {
        play.actors.actor(h).is_none_or(|a| a.killed)
    }

    /// `ObjMure_CheckChildren`: the dead marked, the cut marked.
    fn check_children(&mut self, play: &PlayState) {
        for i in 0..self.get_max_child_spawns() as usize {
            let Some(h) = self.children[i] else { continue };
            if self.children_states[i] == OBJMURE_CHILD_STATE_0 {
                if !Self::gone(play, h) {
                    if play.actors.actor(h).is_some_and(|a| a.flags & ACTOR_FLAG_GRASS_DESTROYED != 0) {
                        self.children_states[i] = OBJMURE_CHILD_STATE_2;
                    }
                } else {
                    self.children_states[i] = OBJMURE_CHILD_STATE_1;
                    self.children[i] = None;
                }
            } else if self.children_states[i] == OBJMURE_CHILD_STATE_2 && Self::gone(play, h) {
                self.children_states[i] = OBJMURE_CHILD_STATE_1;
                self.children[i] = None;
            }
        }
    }

    fn set_child_of(play: &mut PlayState, h: ActorHandle, child: Option<ActorHandle>) {
        if let Some(a) = play.actors.actor_mut(h) {
            a.child = child;
        }
    }

    fn child_of(play: &PlayState, h: ActorHandle) -> Option<ActorHandle> {
        play.actors.actor(h).and_then(|a| a.child)
    }

    /// `ObjMure_CulledState`.
    fn culled_state(&mut self, play: &mut PlayState) {
        if self.actor.projected_pos.z.abs() < Z_CLIP[self.ty as usize] {
            self.action = Action::ActiveState;
            self.actor.flags |= ACTOR_FLAG_UPDATE_CULLING_DISABLED;
            self.spawn_actors(play);
        }
    }

    /// `ObjMure_SetFollowTargets`: each child, by chance `randMax`, follows another at random.
    fn set_follow_targets(&mut self, play: &mut PlayState, rand_max: f32) {
        let max_children = self.get_max_child_spawns();
        for i in 0..max_children as usize {
            let Some(h) = self.children[i] else { continue };
            Self::set_child_of(play, h, None);
            if play.rand.zero_one() <= rand_max {
                let index = (play.rand.zero_one() * (max_children as f32 - 0.5)) as i32 as usize;
                if i != index {
                    Self::set_child_of(play, h, self.children[index]);
                }
            }
        }
    }

    /// `ObjMure_SetChildToFollowPlayer`: the first `idx1` children follow Link (`child` itself);
    /// no one follows them.
    fn set_child_to_follow_player(&mut self, play: &mut PlayState, idx1: i32) {
        let max_children = self.get_max_child_spawns() as usize;
        let mut i2 = 0;
        for i in 0..max_children {
            let Some(h) = self.children[i] else { continue };
            if i2 < idx1 {
                i2 += 1;
                Self::set_child_of(play, h, Some(h));
                for j in 0..max_children {
                    // @bug (game): children[j] may be NULL, read through anyway; none follows.
                    if let Some(hj) = self.children[j]
                        && i != j
                        && Self::child_of(play, hj) == Some(h)
                    {
                        Self::set_child_of(play, hj, None);
                    }
                }
            } else if Self::child_of(play, h) == Some(h) {
                Self::set_child_of(play, h, None);
            }
        }
    }

    /// `ObjMure_GroupBehavior0`: fish and bugs.
    fn group_behavior0(&mut self, play: &mut PlayState) {
        if self.unk_1a4 <= 0 {
            if self.unk_1a6 {
                self.unk_1a6 = false;
                let r = (play.rand.zero_one() * 0.5) + 0.1;
                self.set_follow_targets(play, r);
                self.unk_1a4 = if self.actor.xz_dist_to_player < 60.0 { (play.rand.zero_one() * 5.5) as i16 + 4 } else { (play.rand.zero_one() * 40.5) as i16 + 4 };
            } else {
                self.unk_1a6 = true;
                if self.actor.xz_dist_to_player < 60.0 {
                    self.unk_1a4 = (play.rand.zero_one() * 10.5) as i16 + 4;
                    let r = (play.rand.zero_one() * 0.2) + 0.8;
                    self.set_follow_targets(play, r);
                } else {
                    self.unk_1a4 = (play.rand.zero_one() * 10.5) as i16 + 4;
                    let r = (play.rand.zero_one() * 0.2) + 0.6;
                    self.set_follow_targets(play, r);
                }
            }
        }
        if self.actor.xz_dist_to_player < 120.0 {
            self.unk_1a8 = self.unk_1a8.wrapping_add(1);
        } else {
            self.unk_1a8 = 0;
        }
        if self.unk_1a8 >= 80 {
            self.set_child_to_follow_player(play, 1);
        } else {
            self.set_child_to_follow_player(play, 0);
        }
    }

    /// `ObjMure_GroupBehavior1`: butterflies.
    fn group_behavior1(&mut self, play: &mut PlayState) {
        if self.unk_1a4 <= 0 {
            if self.unk_1a6 {
                self.unk_1a6 = false;
                let r = play.rand.zero_one() * 0.2;
                self.set_follow_targets(play, r);
                self.unk_1a4 = if self.actor.xz_dist_to_player < 60.0 { (play.rand.zero_one() * 5.5) as i16 + 4 } else { (play.rand.zero_one() * 40.5) as i16 + 4 };
            } else {
                self.unk_1a6 = true;
                let r = play.rand.zero_one() * 0.7;
                self.set_follow_targets(play, r);
                self.unk_1a4 = (play.rand.zero_one() * 10.5) as i16 + 4;
            }
        }
        for i in 0..self.get_max_child_spawns() as usize {
            let Some(h) = self.children[i] else { continue };
            if let Some(c) = Self::child_of(play, h)
                && Self::gone(play, c)
            {
                Self::set_child_of(play, h, None);
            }
        }
    }

    /// `ObjMure_ActiveState`.
    fn active_state(&mut self, play: &mut PlayState) {
        self.check_children(play);
        if Z_CLIP[self.ty as usize] + 40.0 <= self.actor.projected_pos.z.abs() {
            self.action = Action::CulledState;
            self.actor.flags &= !ACTOR_FLAG_UPDATE_CULLING_DISABLED;
            self.kill_actors(play);
        } else {
            // sTypeGroupBehaviorFunc.
            match self.ty {
                OBJMURE_TYPE_FISH | OBJMURE_TYPE_BUGS => self.group_behavior0(play),
                OBJMURE_TYPE_BUTTERFLY => self.group_behavior1(play),
                _ => {}
            }
        }
    }
}

impl ActorImpl for ObjMure {
    fn name(&self) -> &'static str {
        PROFILE.name
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    /// `ObjMure_Update`.
    fn update(&mut self, play: &mut PlayState) {
        if self.unk_1a4 > 0 {
            self.unk_1a4 -= 1;
        }
        match self.action {
            Action::InitialAction => self.action = Action::CulledState,
            Action::CulledState => self.culled_state(play),
            Action::ActiveState => self.active_state(play),
        }
    }
    fn render_state(&self) -> RenderState {
        RenderState::of(&self.actor)
    }
    /// No draw (`Obj_Mure_Profile.draw` is NULL).
    fn draw(&self, _rs: &RenderState, _play: &PlayState, _view: &ViewInfo, _out: &mut DrawOut) {}
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
