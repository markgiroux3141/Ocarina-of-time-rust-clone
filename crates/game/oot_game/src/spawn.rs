//! Spawning actors by id (`z_actor.c`): `Actor_Spawn` with the profile from the pack's actor
//! table (`crate::actor_table`), the ported actors' constructors (`Overlays`, registered by the
//! content crate), a `Placeholder` for every actor that isn't ported yet, the spawns of a
//! room's actor list and of the transition actors, and the kills a room change makes.
//!
//! **Init timing.** `Actor_Spawn` adds the actor to its category and runs `Actor_Init`; the
//! actor's own init runs then if its object is loaded, else from `Actor_UpdateAll` once it is
//! (and the actor skips that frame's update). Here the constructor is the init: an actor whose
//! object is still loading waits as an `Uninit` holding its base `Actor` and its constructor.

use std::collections::HashMap;

use glam::Vec3;

use crate::actor::{Actor, Rot};
use crate::actor_ctx::{ACTOR_PLAYER, ACTORCAT_ENEMY, ActorHandle, ActorImpl};
use crate::play::{DrawOut, PlayState, RenderState, ViewInfo};
use crate::scene::{ActorEntry, TRANSITION_ACTOR_PARAMS_INDEX_SHIFT};

/// A ported actor's init: builds it from the base `Actor` that `Actor_Spawn` and `Actor_Init`
/// set up (id, category, flags, params, room, home and world position and rotation).
pub type ActorCtor = fn(Actor, &mut PlayState) -> Box<dyn ActorImpl>;

/// The ported actors' constructors, by `ACTOR_*` id (`gActorOverlayTable`'s code).
#[derive(Default, Clone)]
pub struct Overlays {
    ctors: HashMap<i16, ActorCtor>,
}

impl Overlays {
    pub fn register(&mut self, id: i16, ctor: ActorCtor) {
        self.ctors.insert(id, ctor);
    }

    pub fn get(&self, id: i16) -> Option<ActorCtor> {
        self.ctors.get(&id).copied()
    }

    pub fn is_ported(&self, id: i16) -> bool {
        self.ctors.contains_key(&id)
    }
}

/// An actor whose object hasn't loaded yet: `actor->init != NULL`.
pub struct Uninit {
    pub actor: Actor,
    pub ctor: ActorCtor,
}

impl ActorImpl for Uninit {
    fn name(&self) -> &'static str {
        "(not initialised)"
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    fn update(&mut self, _play: &mut PlayState) {}
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// An actor that isn't ported: it spawns with its real profile (category, flags, object,
/// room) and stays where it was placed, doing nothing. Drawn as a marker when
/// `Debug::placeholders` is on.
///
/// **It can't be targeted** (a deviation): its `InitVars` flags may say targetable
/// (`ACTOR_FLAG_ATTENTION_ENABLED`), but the init that would set its focus, its target mode, or clear the flag
/// again never runs, and it draws nothing. So `ACTOR_FLAG_ATTENTION_ENABLED` is cleared, and the profile's
/// flags are kept in `profile_flags`.
pub struct Placeholder {
    pub actor: Actor,
    pub profile_flags: u32,
}

impl Placeholder {
    /// The "init" of every unported actor.
    pub fn init(mut actor: Actor, _play: &mut PlayState) -> Box<dyn ActorImpl> {
        let profile_flags = actor.flags;
        actor.flags &= !crate::actor::ACTOR_FLAG_ATTENTION_ENABLED;
        Box::new(Placeholder { actor, profile_flags })
    }
}

impl ActorImpl for Placeholder {
    fn name(&self) -> &'static str {
        "Placeholder"
    }
    fn base(&self) -> &Actor {
        &self.actor
    }
    fn base_mut(&mut self) -> &mut Actor {
        &mut self.actor
    }
    fn update(&mut self, _play: &mut PlayState) {}
    fn draw(&self, rs: &RenderState, play: &PlayState, _view: &ViewInfo, out: &mut DrawOut) {
        if play.debug.placeholders {
            let m = crate::play::actor_matrix(rs.pos, rs.rot[1], 1.0);
            out.opa.push(eng_gfx::DrawCmd::new(eng_gfx::MeshKey::named(crate::play::builtin::PLACEHOLDER), m));
        }
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// Why `Actor_Spawn` didn't spawn an actor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpawnFailure {
    /// `actorCtx->total > ACTOR_NUMBER_MAX`.
    TooMany,
    /// No `ActorProfile` for the id (an unset row of the actor table).
    NoProfile,
    /// Its object isn't in a bank (`objBankIndex < 0`), or it's an enemy in a cleared room.
    NoObject,
}

impl PlayState {
    /// `Actor_Spawn`: an actor of `id` at `pos` with rotation `rot` and `params`, into its
    /// category, in the current room. Ported ids get their own init; the rest a `Placeholder`.
    pub fn actor_spawn(&mut self, id: i16, pos: Vec3, rot: [i16; 3], params: i16) -> Result<ActorHandle, SpawnFailure> {
        if self.actors.total() > crate::actor_ctx::ACTOR_NUMBER_MAX {
            return Err(SpawnFailure::TooMany);
        }
        let Some(assets) = self.assets.clone() else { return Err(SpawnFailure::NoProfile) };
        let Some(init) = assets.actors.get(id).and_then(|a| a.init.clone()) else {
            log::warn!("Actor_Spawn: no ActorProfile for {}", assets.actors.name(id));
            return Err(SpawnFailure::NoProfile);
        };
        // Scene_CommandPlayerEntryList sets Player's object to Link's for his age.
        let object = if id == ACTOR_PLAYER { self.link_object_id } else { init.object_id };
        let bank = self.object_ctx.get_index(object);
        if bank.is_none() || (init.category as usize == ACTORCAT_ENEMY && self.flags.get_clear(self.room_ctx.cur.num)) {
            log::debug!("Actor_Spawn: {} has no object bank (object {object})", assets.actors.name(id));
            return Err(SpawnFailure::NoObject);
        }
        // Actor_Spawn's copy of the profile, then Actor_Init.
        let mut a = Actor::new(pos, rot[1]);
        a.id = init.id;
        a.category = init.category as usize;
        a.flags = init.flags;
        a.obj_bank_index = bank;
        a.room = self.room_ctx.cur.num;
        a.home_rot = Rot { x: rot[0], y: rot[1], z: rot[2] };
        a.world_rot = a.home_rot;
        a.shape_rot = a.world_rot;
        a.params = params;
        a.focus_pos = a.world_pos;
        a.xyz_dist_to_player_sq = f32::MAX;
        a.teleported = true;
        let ctor = assets.overlays.get(id).unwrap_or(Placeholder::init);
        if !self.object_ctx.is_loaded(bank.unwrap()) {
            return self.spawn(Box::new(Uninit { actor: a, ctor })).ok_or(SpawnFailure::TooMany);
        }
        self.init_children.push(Vec::new());
        let actor = ctor(a, self);
        let children = self.init_children.pop().unwrap_or_default();
        let h = self.spawn(actor).ok_or(SpawnFailure::TooMany)?;
        self.link_init_children(h, children);
        Ok(h)
    }

    /// An actor's init (`ctor`) whose handle is `h` already (an `Uninit` whose object came in):
    /// the children it spawns get it as their parent.
    pub(crate) fn run_init(&mut self, h: ActorHandle, actor: Actor, ctor: crate::spawn::ActorCtor) -> Box<dyn ActorImpl> {
        self.init_children.push(Vec::new());
        let category = actor.category;
        let init = ctor(actor, self);
        let children = self.init_children.pop().unwrap_or_default();
        self.link_init_children(h, children);
        // Actor_ChangeCategory in the init (`En_Sw`'s Skullwalltula, an enemy among the NPCs):
        // to the head of its new list, which `Actor_UpdateAll` then walks on in.
        if init.base().category != category {
            self.actors.change_category(h, init.base().category);
        }
        init
    }

    /// `spawnedActor->parent = parent` for the children an init spawned as its own.
    fn link_init_children(&mut self, parent: ActorHandle, children: Vec<ActorHandle>) {
        for c in children {
            if let Some(a) = self.actors.actor_mut(c) {
                a.parent = Some(parent);
            }
        }
    }

    /// `Actor_SpawnAsChild`: spawned by the updating actor (`parent` is its base, taken out of
    /// its slot), which becomes the new actor's `parent`, and the new actor its `child`. The
    /// child's init runs before the link is made, as in the game; it takes the parent's room.
    pub fn actor_spawn_as_child(&mut self, parent: &mut Actor, id: i16, pos: Vec3, rot: [i16; 3], params: i16) -> Result<ActorHandle, SpawnFailure> {
        let h = self.actor_spawn(id, pos, rot, params)?;
        parent.child = Some(h);
        // From the parent's init the parent isn't in the actor context yet: linked once it is.
        let in_init = match self.init_children.last_mut() {
            Some(v) => {
                v.push(h);
                true
            }
            None => false,
        };
        if let Some(a) = self.actors.actor_mut(h) {
            if !in_init {
                a.parent = self.cur_actor;
            }
            if a.room >= 0 {
                a.room = parent.room;
            }
        }
        Ok(h)
    }

    /// `Actor_SpawnEntry`.
    pub fn actor_spawn_entry(&mut self, e: &ActorEntry) -> Result<ActorHandle, SpawnFailure> {
        let p = Vec3::new(e.pos[0] as f32, e.pos[1] as f32, e.pos[2] as f32);
        self.actor_spawn(e.id, p, e.rot, e.params)
    }

    /// `Actor_SpawnTransitionActors`: every transition actor with a side in the current or
    /// previous room that isn't already spawned, with its list index in the params' top bits.
    /// A spawned entry's id is negated (its destroy negates it back).
    pub fn spawn_transition_actors(&mut self) {
        let (cur, prev) = (self.room_ctx.cur.num, self.room_ctx.prev.num);
        for i in 0..self.transi_actors.len() {
            let t = self.transi_actors[i];
            if t.id < 0 {
                continue;
            }
            let near = |r: i8| r >= 0 && (r == cur || r == prev);
            if near(t.sides[0].0) || near(t.sides[1].0) {
                let pos = Vec3::new(t.pos[0] as f32, t.pos[1] as f32, t.pos[2] as f32);
                let params = ((i as i32) << TRANSITION_ACTOR_PARAMS_INDEX_SHIFT) + t.params as i32;
                if let Err(e) = self.actor_spawn(t.id & 0x1FFF, pos, [0, t.rot_y, 0], params as i16) {
                    log::debug!("transition actor {i}: {e:?}");
                }
                self.transi_actors[i].id = -t.id;
            }
        }
    }

    /// Spawns the room's actor list (`numSetupActors`), as `Actor_UpdateAll` does first thing.
    pub(crate) fn spawn_setup_actors(&mut self) {
        for e in std::mem::take(&mut self.setup_actors) {
            if let Err(err) = self.actor_spawn_entry(&e) {
                log::debug!("setup actor {:#06x}: {err:?}", e.id);
            }
        }
    }

    /// `func_80031B14`: kills every actor in neither the current nor the previous room
    /// (`isDrawn` isn't tracked: they're deleted at once, destroy first).
    pub fn kill_actors_outside_rooms(&mut self) {
        let (cur, prev) = (self.room_ctx.cur.num, self.room_ctx.prev.num);
        for h in self.actors.all() {
            let Some(a) = self.actors.actor(h) else { continue };
            if a.room >= 0 && a.room != cur && a.room != prev {
                self.delete_actor(h);
            }
        }
        self.flags.temp_clear = 0;
        self.flags.temp_swch &= 0xFF_FFFF;
    }

    /// `Actor_KillAllWithMissingObject`: `Actor_Kill` for every actor whose object bank was dropped.
    pub fn kill_actors_without_objects(&mut self) {
        for h in self.actors.all() {
            if let Some(a) = self.actors.actor_mut(h)
                && let Some(b) = a.obj_bank_index
                && !self.object_ctx.is_loaded(b)
            {
                a.kill();
            }
        }
    }

    /// `Actor_Delete`: destroy, then remove from its category and free it.
    pub fn delete_actor(&mut self, h: ActorHandle) {
        if let Some(mut a) = self.actors.take(h) {
            a.destroy(self);
            self.actors.put_back(h, a);
        }
        self.actors.remove(h);
        if self.player == Some(h) {
            self.player = None;
        }
    }
}

/// `actorCtx.flags`: the scene's switch, chest, clear and collectible flags.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SceneFlags {
    pub swch: u32,
    pub temp_swch: u32,
    pub chest: u32,
    pub clear: u32,
    pub temp_clear: u32,
    pub collect: u32,
    pub temp_collect: u32,
}

impl SceneFlags {
    /// `Flags_GetSwitch`.
    pub fn get_switch(&self, flag: i32) -> bool {
        match flag {
            0..=0x1F => self.swch & (1 << flag) != 0,
            0x20..=0x3F => self.temp_swch & (1 << (flag - 0x20)) != 0,
            _ => false,
        }
    }

    /// `Flags_SetSwitch`.
    pub fn set_switch(&mut self, flag: i32) {
        match flag {
            0..=0x1F => self.swch |= 1 << flag,
            0x20..=0x3F => self.temp_swch |= 1 << (flag - 0x20),
            _ => {}
        }
    }

    /// `Flags_GetCollectible`.
    pub fn get_collectible(&self, flag: i32) -> bool {
        match flag {
            0..=0x1F => self.collect & (1 << flag) != 0,
            0x20..=0x3F => self.temp_collect & (1 << (flag - 0x20)) != 0,
            _ => false,
        }
    }

    /// `Flags_SetCollectible` (flag 0 is "none").
    pub fn set_collectible(&mut self, flag: i32) {
        match flag {
            1..=0x1F => self.collect |= 1 << flag,
            0x20..=0x3F => self.temp_collect |= 1 << (flag - 0x20),
            _ => {}
        }
    }

    /// `Flags_GetTreasure`.
    pub fn get_treasure(&self, flag: i32) -> bool {
        self.chest & (1u32 << (flag & 0x1F)) != 0
    }

    /// `Flags_SetTreasure`.
    pub fn set_treasure(&mut self, flag: i32) {
        self.chest |= 1u32 << (flag & 0x1F);
    }

    /// `Flags_SetClear`.
    pub fn set_clear(&mut self, room: i8) {
        match room {
            0..=0x1F => self.clear |= 1 << room,
            0x20..=0x3F => self.temp_clear |= 1 << (room - 0x20),
            _ => {}
        }
    }

    /// `Flags_GetTempClear`.
    pub fn get_temp_clear(&self, room: i8) -> bool {
        (0..=0x1F).contains(&room) && self.temp_clear & (1 << room) != 0
    }

    /// `Flags_GetClear`.
    pub fn get_clear(&self, room: i8) -> bool {
        match room {
            0..=0x1F => self.clear & (1 << room) != 0,
            0x20..=0x3F => self.temp_clear & (1 << (room - 0x20)) != 0,
            _ => false,
        }
    }
}
