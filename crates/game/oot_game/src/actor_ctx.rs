//! The actor system: `ActorContext` from `z_actor.c`.
//!
//! - **Ownership** (docs/adr/0007-actor-ownership.md): actors live in a generational arena.
//!   A `ActorHandle` is a slot index plus the slot's generation, so a handle to a deleted actor
//!   never reaches the slot's next occupant.
//! - **Actors embed the base `Actor`**, as the decomp's actor structs start with one. The
//!   system reaches it through `ActorImpl::base`; each type's own code uses its field directly.
//! - **Categories:** `ACTORCAT_*` lists of handles. `Actor_AddToCategory` inserts at the head,
//!   so each list is newest first, the order `Actor_UpdateAll` and `Actor_DrawAll` visit.
//! - **Updating:** during its update an actor is taken out of its slot (`take`). It gets
//!   `&mut PlayState`, which can reach every other actor by handle, and goes back afterwards
//!   (`put_back`). Looking itself up meanwhile finds nothing, as the decomp never reaches an
//!   actor through the lists from inside its own update.

use std::any::Any;

use glam::Vec3;

use crate::actor::Actor;
use crate::play::{DrawOut, PlayState, RenderState, ViewInfo};

pub const ACTORCAT_SWITCH: usize = 0;
pub const ACTORCAT_BG: usize = 1;
pub const ACTORCAT_PLAYER: usize = 2;
pub const ACTORCAT_EXPLOSIVE: usize = 3;
pub const ACTORCAT_NPC: usize = 4;
pub const ACTORCAT_ENEMY: usize = 5;
pub const ACTORCAT_PROP: usize = 6;
pub const ACTORCAT_ITEMACTION: usize = 7;
pub const ACTORCAT_MISC: usize = 8;
pub const ACTORCAT_BOSS: usize = 9;
pub const ACTORCAT_DOOR: usize = 10;
pub const ACTORCAT_CHEST: usize = 11;
pub const ACTORCAT_MAX: usize = 12;
/// `ACTOR_NUMBER_MAX`: `Actor_Spawn` refuses more.
pub const ACTOR_NUMBER_MAX: usize = 200;

/// `ACTOR_*` ids of the ported actors (`include/tables/actor_table.h`).
pub const ACTOR_PLAYER: i16 = 0x0000;
pub const ACTOR_BG_YDAN_HASI: i16 = 0x0050;
/// Not a game actor: the sandbox's dummy Z-target.
pub const ACTOR_SANDBOX_DUMMY_TARGET: i16 = -2;

/// An actor type's `ActorInit`: its id, category, initial flags and object dependency. (The
/// functions are the `ActorImpl` methods.)
#[derive(Debug, Clone, Copy)]
pub struct ActorProfile {
    pub id: i16,
    /// The overlay's name, e.g. `Bg_Ydan_Hasi`.
    pub name: &'static str,
    pub category: usize,
    pub flags: u32,
    /// The object file its assets are in (`OBJECT_*`).
    pub object: &'static str,
}

impl ActorProfile {
    /// What `Actor_Spawn` copies from the profile into the actor.
    pub fn apply(&self, a: &mut Actor) {
        a.id = self.id;
        a.category = self.category;
        a.flags = self.flags;
    }
}

/// A reference to an actor that stays valid (and safe) after the actor is deleted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ActorHandle {
    index: u32,
    generation: u32,
}

/// What the framework (the camera, the target context, `Actor_UpdateAll`) reads from Player
/// (`GET_PLAYER(play)`). Player implements it; everything else is Player's own business.
pub trait PlayerIface {
    /// `LINK_IS_ADULT`.
    fn adult(&self) -> bool;
    /// `stateFlags1`.
    fn state_flags1(&self) -> u32;
    /// `stateFlags2`.
    fn state_flags2(&self) -> u32;
    /// `unk_664`: the actor Player is locked on to.
    fn target(&self) -> Option<ActorHandle>;
    /// `unk_66C`: the Z timer.
    fn target_timer(&self) -> i16;
    /// `unk_84B[unk_846]`: the stick direction class.
    fn stick_dir(&self) -> i8;
    /// `actor.focus.pos`: the head.
    fn focus(&self) -> Vec3;
    /// `speedXZ`, for the follow camera.
    fn speed_xz(&self) -> f32;
    /// `targetActor` and `targetActorDistance`: the nearest actor offering to talk this frame
    /// (`func_8002F1C4`), reset at the end of every Player update.
    fn talk_target(&self) -> (Option<ActorHandle>, f32);
    /// `func_8002F1C4`'s write: `targetActor`, `targetActorDistance`, `exchangeItemId`.
    fn set_talk_target(&mut self, actor: ActorHandle, distance: f32, exchange_item: u8);
    /// Player's part of `Player_InCsMode` (`Player_InBlockingCsMode` without the transition
    /// trigger, or `unk_6AD == 4`).
    fn in_cs_mode(&self) -> bool;
    /// `heldActor != NULL`.
    fn holds_actor(&self) -> bool;
    /// `getItemDirection`: how squarely the last `GI_NONE` offer this frame faced Link.
    fn get_item_direction(&self) -> i16;
    /// `func_8002F434`'s write: `getItemId`, `interactRangeActor`, `getItemDirection`.
    fn set_get_item(&mut self, actor: ActorHandle, get_item_id: i16, direction: i16);
    /// `func_8002F698`'s write: `unk_8A0` (the extra damage), `unk_8A1` (the kind: 1 a push,
    /// 2 a knockdown, 3 a shock), `unk_8A2` (the yaw), `unk_8A4` (the speed), `unk_8A8` (the
    /// upward speed).
    fn set_knockback(&mut self, damage: u8, kind: u8, yaw: i16, speed: f32, vy: f32);
    /// `invincibilityTimer`.
    fn invincibility_timer(&self) -> i8;
    /// An actor's write of `stateFlags2`: `set` bits on, then `clear` bits off (`En_Ossan`'s
    /// `PLAYER_STATE2_29`, Link hidden while he browses the shelves).
    fn change_state_flags2(&mut self, set: u32, clear: u32);
    /// `Player_SetEquipmentData` from `save` (the pause menu's closing runs it).
    fn set_equipment_data(&mut self, data: &crate::data::GameData, save: &crate::save::SaveContext);
    /// `func_8002DF38` / `func_8002DF54`'s writes: `csMode`, `unk_448` (the actor the mode is
    /// about) and `doorBgCamIndex`.
    fn set_cs_mode(&mut self, cs_mode: u8, actor: Option<ActorHandle>, door_bg_cam_index: i16);
    /// `csMode`.
    fn cs_mode(&self) -> u8;
    /// `bodyPartsPos[i]` (`PLAYER_BODYPART_*`), from the last draw.
    fn body_part(&self, i: usize) -> Vec3;
    /// `naviTextId`: what Navi says when C-Up talks to her (negative: at once), set by her
    /// update and cleared at the end of every Player update.
    fn navi_text_id(&self) -> i16;
    fn set_navi_text_id(&mut self, id: i16);
    /// `naviActor`: the fairy `Player_Init` spawned.
    fn navi_actor(&self) -> Option<ActorHandle>;
    /// `unk_89E`: the floor's footstep (`SurfaceType_GetSfxId`'s offset), which the crawl's
    /// camera plays.
    fn unk_89e(&self) -> u16 {
        0
    }
    /// `currentBoots` (`PLAYER_BOOTS_*`).
    fn current_boots(&self) -> u8 {
        0
    }
    /// An outside write of `stateFlags1`: `set` bits on, then `clear` bits off (`Camera_Finish`
    /// and `Camera_Demo5`'s `PLAYER_STATE1_29`).
    fn change_state_flags1(&mut self, _set: u32, _clear: u32) {}
}

/// `PLAYER_BOOTS_IRON` (`z64player.h`).
pub const PLAYER_BOOTS_IRON: u8 = 1;

/// `PLAYER_BODYPART_*` (`z64player.h`) the other actors read.
pub const PLAYER_BODYPART_WAIST: usize = 0;
pub const PLAYER_BODYPART_HEAD: usize = 7;
pub const PLAYER_BODYPART_HAT: usize = 8;

/// An actor type: its data (with the base `Actor` inside) and its `ActorInit` functions.
pub trait ActorImpl: Any {
    /// The actor's name (the overlay's, e.g. `Bg_Ydan_Hasi`).
    fn name(&self) -> &'static str;
    fn base(&self) -> &Actor;
    fn base_mut(&mut self) -> &mut Actor;
    /// `ActorInit.update`.
    fn update(&mut self, play: &mut PlayState);
    /// This actor's share of `AnimationContext_Update`: its queued animation requests.
    fn animation_update(&mut self) {}
    /// The part of `ActorInit.draw` that changes the actor (Player's foot IK writes into its
    /// joint table), run once per game frame.
    fn draw_update(&mut self, _play: &mut PlayState) {}
    /// The audio calls of `ActorInit.draw` (`EnRiverSound_Draw`'s), where `Actor_DrawAll` makes
    /// them: right after the actor's `projectedPos` and its `sfx`.
    fn draw_sfx(&mut self, _play: &mut PlayState) {}
    /// What the renderer blends between game frames.
    fn render_state(&self) -> RenderState {
        RenderState::of(self.base())
    }
    /// `ActorInit.draw`: submit this frame's draws, from the blended state `rs`.
    fn draw(&self, _rs: &RenderState, _play: &PlayState, _view: &ViewInfo, _out: &mut DrawOut) {}
    /// `ActorInit.destroy`.
    fn destroy(&mut self, _play: &mut PlayState) {}
    /// `GET_PLAYER`.
    fn as_player(&self) -> Option<&dyn PlayerIface> {
        None
    }
    fn as_player_mut(&mut self) -> Option<&mut dyn PlayerIface> {
        None
    }
    /// The actor's collider `id`, as it registered it with `CollisionCheck_Set*`: the checks
    /// take it out and put it back (`crate::collision_check`).
    fn collider_mut(&mut self, _id: u8) -> Option<crate::collision_check::ColliderMut<'_>> {
        None
    }
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl dyn ActorImpl {
    /// The box as `Box<dyn Any>`, to take a concrete type out.
    pub fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

struct Slot {
    generation: u32,
    /// `None` while free, or while the actor is taken out for its update.
    actor: Option<Box<dyn ActorImpl>>,
    live: bool,
}

/// `ActorContext`: the arena and the category lists.
#[derive(Default)]
pub struct ActorContext {
    slots: Vec<Slot>,
    free: Vec<u32>,
    /// `actorLists[ACTORCAT_MAX]`, newest first.
    lists: [Vec<ActorHandle>; ACTORCAT_MAX],
}

impl ActorContext {
    /// `actorCtx->total`.
    pub fn total(&self) -> usize {
        self.lists.iter().map(|l| l.len()).sum()
    }

    /// Adds a constructed (and initialised) actor: `Actor_Spawn`'s allocation and
    /// `Actor_AddToCategory` (at the head of its category's list). `None` past
    /// `ACTOR_NUMBER_MAX`.
    pub fn insert(&mut self, actor: Box<dyn ActorImpl>) -> Option<ActorHandle> {
        if self.total() > ACTOR_NUMBER_MAX {
            log::warn!("Actor_Spawn: too many actors, {} not spawned", actor.name());
            return None;
        }
        let cat = actor.base().category.min(ACTORCAT_MAX - 1);
        let h = match self.free.pop() {
            Some(i) => {
                let s = &mut self.slots[i as usize];
                s.generation += 1;
                s.actor = Some(actor);
                s.live = true;
                ActorHandle { index: i, generation: s.generation }
            }
            None => {
                self.slots.push(Slot { generation: 0, actor: Some(actor), live: true });
                ActorHandle { index: self.slots.len() as u32 - 1, generation: 0 }
            }
        };
        self.lists[cat].insert(0, h);
        Some(h)
    }

    fn slot(&self, h: ActorHandle) -> Option<&Slot> {
        self.slots.get(h.index as usize).filter(|s| s.live && s.generation == h.generation)
    }

    fn slot_mut(&mut self, h: ActorHandle) -> Option<&mut Slot> {
        self.slots.get_mut(h.index as usize).filter(|s| s.live && s.generation == h.generation)
    }

    /// Is the handle's actor still there (not deleted)? True while it's taken out too.
    pub fn exists(&self, h: ActorHandle) -> bool {
        self.slot(h).is_some()
    }

    pub fn get(&self, h: ActorHandle) -> Option<&dyn ActorImpl> {
        self.slot(h)?.actor.as_deref()
    }

    pub fn get_mut(&mut self, h: ActorHandle) -> Option<&mut (dyn ActorImpl + 'static)> {
        self.slot_mut(h)?.actor.as_deref_mut()
    }

    /// The base `Actor` of `h`.
    pub fn actor(&self, h: ActorHandle) -> Option<&Actor> {
        self.get(h).map(|a| a.base())
    }

    pub fn actor_mut(&mut self, h: ActorHandle) -> Option<&mut Actor> {
        self.get_mut(h).map(|a| a.base_mut())
    }

    /// `h` as its concrete type.
    pub fn downcast<T: ActorImpl>(&self, h: ActorHandle) -> Option<&T> {
        self.get(h)?.as_any().downcast_ref()
    }

    pub fn downcast_mut<T: ActorImpl>(&mut self, h: ActorHandle) -> Option<&mut T> {
        self.get_mut(h)?.as_any_mut().downcast_mut()
    }

    /// Takes the actor out of its slot for its update.
    pub fn take(&mut self, h: ActorHandle) -> Option<Box<dyn ActorImpl>> {
        self.slot_mut(h)?.actor.take()
    }

    /// Puts an actor taken with `take` back.
    pub fn put_back(&mut self, h: ActorHandle, actor: Box<dyn ActorImpl>) {
        if let Some(s) = self.slot_mut(h) {
            s.actor = Some(actor);
        }
    }

    /// `Actor_Delete`'s bookkeeping: `Actor_RemoveFromCategory` and freeing the slot. Returns
    /// the actor for its `destroy`.
    pub fn remove(&mut self, h: ActorHandle) -> Option<Box<dyn ActorImpl>> {
        let s = self.slot_mut(h)?;
        let a = s.actor.take();
        s.live = false;
        self.free.push(h.index);
        for l in &mut self.lists {
            l.retain(|&x| x != h);
        }
        a
    }

    /// `actorLists[cat]`, newest first.
    pub fn category(&self, cat: usize) -> &[ActorHandle] {
        &self.lists[cat]
    }

    /// Every actor in `Actor_UpdateAll` order: categories in order, each newest first.
    pub fn all(&self) -> Vec<ActorHandle> {
        self.lists.iter().flatten().copied().collect()
    }

    /// The first actor of a category with this id (`Actor_Find`).
    pub fn find(&self, id: i16, cat: usize) -> Option<ActorHandle> {
        self.lists[cat].iter().copied().find(|&h| self.actor(h).is_some_and(|a| a.id == id))
    }
}

/// `func_8002F698`: an actor knocks Player back. Player takes it in its next update
/// (`func_808382DC`): `kind` 1 a push, 2 a knockdown, 3 a shock, at `speed` along `yaw` with
/// `vy` upwards, `damage` added to what the frame's collisions did.
pub fn func_8002f698(play: &mut PlayState, speed: f32, yaw: i16, vy: f32, kind: u8, damage: u8) {
    if let Some(pi) = play.player.and_then(|h| play.actors.get_mut(h)).and_then(|p| p.as_player_mut()) {
        pi.set_knockback(damage, kind, yaw, speed, vy);
    }
}

/// `func_8002F6D4`: a knockdown (kind 2).
pub fn func_8002f6d4(play: &mut PlayState, speed: f32, yaw: i16, vy: f32, damage: u8) {
    func_8002f698(play, speed, yaw, vy, 2, damage);
}

/// `func_8002F71C`: a knockdown with no damage of its own.
pub fn func_8002f71c(play: &mut PlayState, speed: f32, yaw: i16, vy: f32) {
    func_8002f6d4(play, speed, yaw, vy, 0);
}

/// `func_8002F758`: a push (kind 1).
pub fn func_8002f758(play: &mut PlayState, speed: f32, yaw: i16, vy: f32, damage: u8) {
    func_8002f698(play, speed, yaw, vy, 1, damage);
}

/// `func_8002F7A0`: a push with no damage of its own.
pub fn func_8002f7a0(play: &mut PlayState, speed: f32, yaw: i16, vy: f32) {
    func_8002f758(play, speed, yaw, vy, 0);
}

/// Where the updating actor's sounds are: `&this->actor.projectedPos` (`play.cur_actor`;
/// `gSfxDefaultPos` outside an actor's turn).
pub fn cur_sfx_pos(play: &PlayState) -> crate::audio::sfx::SfxPos {
    play.cur_actor.map(crate::audio::sfx::SfxPos::Actor).unwrap_or(crate::audio::sfx::SfxPos::Default)
}

/// `func_8002F7DC`: `Audio_PlaySfxGeneral` at the actor's `projectedPos`, with the defaults.
pub fn func_8002f7dc(play: &mut PlayState, actor: ActorHandle, sfx_id: u16) {
    use crate::audio::sfx::{SfxF32, SfxPos, SfxS8};
    play.audio.play_sfx_general(sfx_id, SfxPos::Actor(actor), 4, SfxF32::One, SfxF32::One, SfxS8::Zero);
}

/// `Audio_PlayActorSfx2`: `func_80078914` at the updating actor's `projectedPos`.
pub fn audio_play_actor_sfx2(play: &mut PlayState, sfx_id: u16) {
    let pos = cur_sfx_pos(play);
    play.audio.func_80078914(pos, sfx_id);
}

/// `func_8002F850`: a bounce: `NA_SE_EV_BOMB_BOUND`, then the floor's footstep (the water's
/// when the actor is in water: shallow under 20) at the updating actor.
pub fn func_8002f850(play: &mut PlayState, actor: &Actor) {
    use crate::actor::BGCHECKFLAG_WATER;
    use crate::audio::sfx::{NA_SE_EV_BOMB_BOUND, NA_SE_PL_WALK_WATER0, NA_SE_PL_WALK_WATER1, SFX_FLAG};
    use crate::surface::SurfaceType;
    let sfx_id = if actor.bg_check_flags & BGCHECKFLAG_WATER != 0 {
        if actor.y_dist_to_water < 20.0 { NA_SE_PL_WALK_WATER0 - SFX_FLAG } else { NA_SE_PL_WALK_WATER1 - SFX_FLAG }
    } else {
        // SurfaceType_GetSfxId (the C reads through a NULL floor: type 0 here).
        play.audio.tables.surface_sfx_id(actor.floor_poly.map(|p| play.col.sfx_type(p)).unwrap_or(0))
    };
    let pos = cur_sfx_pos(play);
    play.audio.func_80078914(pos, NA_SE_EV_BOMB_BOUND);
    play.audio.func_80078914(pos, sfx_id.wrapping_add(SFX_FLAG));
}
