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

/// An actor type's `ActorProfile`: its id, category, initial flags and object dependency. (The
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

#[cfg(test)]
impl ActorHandle {
    /// A handle for the crate's tests that need one without an actor context.
    pub(crate) fn for_test(index: u32) -> ActorHandle {
        ActorHandle { index, generation: 0 }
    }
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
    /// `focusActor`: the actor Player is locked on to.
    fn target(&self) -> Option<ActorHandle>;
    /// `zTargetActiveTimer`: the Z timer.
    fn target_timer(&self) -> i16;
    /// `controlStickDirections[controlStickDataIndex]`: the stick direction class.
    fn stick_dir(&self) -> i8;
    /// `actor.focus.pos`: the head.
    fn focus(&self) -> Vec3;
    /// `speedXZ`, for the follow camera.
    fn speed_xz(&self) -> f32;
    /// `targetActor` and `targetActorDistance`: the nearest actor offering to talk this frame
    /// (`Actor_OfferTalkExchange`), reset at the end of every Player update.
    fn talk_target(&self) -> (Option<ActorHandle>, f32);
    /// `Actor_OfferTalkExchange`'s write: `targetActor`, `targetActorDistance`, `exchangeItemId`.
    fn set_talk_target(&mut self, actor: ActorHandle, distance: f32, exchange_item: u8);
    /// Player's part of `Player_InCsMode` (`Player_InBlockingCsMode` without the transition
    /// trigger, or `unk_6AD == 4`).
    fn in_cs_mode(&self) -> bool;
    /// What `Player_GetEnvironmentalHazard` reads of Player: `underwaterTimer`, `currentBoots`,
    /// `currentTunic`, and whether he's on the ground (`BGCHECKFLAG_GROUND`).
    fn env_hazard_state(&self) -> (i16, u8, u8, bool) {
        (0, 0, 0, true)
    }
    /// `heldActor != NULL`.
    fn holds_actor(&self) -> bool;
    /// `getItemDirection`: how squarely the last `GI_NONE` offer this frame faced Link.
    fn get_item_direction(&self) -> i16;
    /// `Actor_OfferGetItem`'s write: `getItemId`, `interactRangeActor`, `getItemDirection`.
    fn set_get_item(&mut self, actor: ActorHandle, get_item_id: i16, direction: i16);
    /// `Actor_SetPlayerKnockback`'s write: `knockbackDamage` (the extra damage), `knockbackType` (the kind: 1 a push,
    /// 2 a knockdown, 3 a shock), `knockbackRot` (the yaw), `knockbackSpeed` (the speed), `knockbackYVelocity` (the
    /// upward speed).
    fn set_knockback(&mut self, damage: u8, kind: u8, yaw: i16, speed: f32, vy: f32);
    /// `invincibilityTimer`.
    fn invincibility_timer(&self) -> i8;
    /// An actor's write of `stateFlags2`: `set` bits on, then `clear` bits off (`En_Ossan`'s
    /// `PLAYER_STATE2_29`, Link hidden while he browses the shelves).
    fn change_state_flags2(&mut self, set: u32, clear: u32);
    /// `Player_SetEquipmentData` from `save` (the pause menu's closing runs it).
    fn set_equipment_data(&mut self, data: &crate::data::GameData, save: &crate::save::SaveContext);
    /// `Player_SetCsAction` / `Player_SetCsActionWithHaltedActors`'s writes: `csMode`, `csActor` (the actor the mode is
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
    /// `floorSfxOffset`: the floor's footstep (`SurfaceType_GetSfxOffset`'s offset), which the crawl's
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
    /// `meleeWeaponState` (0 none, 1 swinging, -1 the swing's end).
    fn melee_weapon_state(&self) -> i8 {
        0
    }
    /// `currentShield` (`PLAYER_SHIELD_*`).
    fn current_shield(&self) -> u8 {
        0
    }
    /// `shieldMf`: the shield's matrix, in hand or on the back, from the last draw.
    fn shield_mf(&self) -> crate::sys_matrix::MtxF {
        crate::sys_matrix::MtxF::IDENTITY
    }
}

/// `PLAYER_BOOTS_IRON` (`player.h`).
pub const PLAYER_BOOTS_IRON: u8 = 1;

/// `PLAYER_BODYPART_*` (`player.h`) the other actors read.
pub const PLAYER_BODYPART_WAIST: usize = 0;
/// `PLAYER_BODYPART_MAX`.
pub const PLAYER_BODYPART_MAX: usize = 18;
pub const PLAYER_BODYPART_HEAD: usize = 7;
pub const PLAYER_BODYPART_HAT: usize = 8;

/// An actor type: its data (with the base `Actor` inside) and its `ActorProfile` functions.
pub trait ActorImpl: Any {
    /// The actor's name (the overlay's, e.g. `Bg_Ydan_Hasi`).
    fn name(&self) -> &'static str;
    fn base(&self) -> &Actor;
    fn base_mut(&mut self) -> &mut Actor;
    /// `ActorProfile.update`.
    fn update(&mut self, play: &mut PlayState);
    /// This actor's share of `AnimTaskQueue_Update`: its queued animation requests.
    fn animation_update(&mut self) {}
    /// The part of `ActorProfile.draw` that changes the actor (Player's foot IK writes into its
    /// joint table), run once per game frame.
    fn draw_update(&mut self, _play: &mut PlayState) {}
    /// The audio calls of `ActorProfile.draw` (`EnRiverSound_Draw`'s), where `Actor_DrawAll` makes
    /// them: right after the actor's `projectedPos` and its `sfx`.
    fn draw_sfx(&mut self, _play: &mut PlayState) {}
    /// What the renderer blends between game frames.
    fn render_state(&self) -> RenderState {
        RenderState::of(self.base())
    }
    /// `ActorProfile.draw`: submit this frame's draws, from the blended state `rs`.
    fn draw(&self, _rs: &RenderState, _play: &PlayState, _view: &ViewInfo, _out: &mut DrawOut) {}
    /// `ActorProfile.destroy`.
    fn destroy(&mut self, _play: &mut PlayState) {}
    /// What `Effect_Ss_En_Fire` reads for a flame on body part `i` (`firePos`): the actor's
    /// table at 0x14C, `Vec3s` with flag 0x8000 else `Vec3f`. An actor with none there has
    /// zeroes (the zeroed actor memory, as `En_Dekubaba`'s unused fields).
    fn effect_fire_pos(&self, _i: usize, _vec3s: bool) -> Vec3 {
        Vec3::ZERO
    }
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
    /// A DynaPoly actor's `dyna.bgId`, for `DynaPoly_UnsetAllInteractFlags` after its update.
    /// `None` for an actor with no bg actor (or none that reads its interact flags).
    fn dyna_bg_id(&self) -> Option<u16> {
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
    /// `freezeFlashTimer`: an enemy's finishing blow (`Enemy_StartFinishingBlow`) stops play for
    /// four frames, with a white flash (`Play_Update`).
    pub freeze_flash_timer: u8,
    /// `unk_02`: the hammer's shock wave (`func_80842A28`), counted down by `Actor_UpdateAll`; it
    /// stuns enemies that check it. Nothing sets it here (the hammer isn't ported).
    pub unk_02: u8,
}

/// `PLAYER_STATE1_*` (`player.h`) that freeze actors' updates (`sCategoryFreezeMasks`).
pub const PLAYER_STATE1_TALKING: u32 = 1 << 6;
pub const PLAYER_STATE1_DEAD: u32 = 1 << 7;
pub const PLAYER_STATE1_10: u32 = 1 << 10;
pub const PLAYER_STATE1_28: u32 = 1 << 28;
pub const PLAYER_STATE1_29: u32 = 1 << 29;

/// `sCategoryFreezeMasks` (`z_actor.c`): by category, the Player states that freeze its actors
/// (`Actor_UpdateAll` only resets their damage).
pub const S_CATEGORY_FREEZE_MASKS: [u32; ACTORCAT_MAX] = [
    // ACTORCAT_SWITCH
    PLAYER_STATE1_TALKING | PLAYER_STATE1_DEAD | PLAYER_STATE1_28,
    // ACTORCAT_BG
    PLAYER_STATE1_TALKING | PLAYER_STATE1_DEAD | PLAYER_STATE1_28,
    // ACTORCAT_PLAYER
    0,
    // ACTORCAT_EXPLOSIVE
    PLAYER_STATE1_TALKING | PLAYER_STATE1_DEAD | PLAYER_STATE1_10 | PLAYER_STATE1_28,
    // ACTORCAT_NPC
    PLAYER_STATE1_DEAD,
    // ACTORCAT_ENEMY
    PLAYER_STATE1_TALKING | PLAYER_STATE1_DEAD | PLAYER_STATE1_28 | PLAYER_STATE1_29,
    // ACTORCAT_PROP
    PLAYER_STATE1_DEAD | PLAYER_STATE1_28,
    // ACTORCAT_ITEMACTION
    0,
    // ACTORCAT_MISC
    PLAYER_STATE1_TALKING | PLAYER_STATE1_DEAD | PLAYER_STATE1_28 | PLAYER_STATE1_29,
    // ACTORCAT_BOSS
    PLAYER_STATE1_TALKING | PLAYER_STATE1_DEAD | PLAYER_STATE1_10 | PLAYER_STATE1_28,
    // ACTORCAT_DOOR
    0,
    // ACTORCAT_CHEST
    PLAYER_STATE1_TALKING | PLAYER_STATE1_DEAD | PLAYER_STATE1_28,
];

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

    /// `Actor_ChangeCategory`'s lists: `h` out of its category's list and onto the head of
    /// `category`'s (`Actor_RemoveFromCategory`, `Actor_AddToCategory`). The caller sets the base's
    /// `category` (the actor may be out for its update).
    pub fn change_category(&mut self, h: ActorHandle, category: usize) {
        for l in self.lists.iter_mut() {
            l.retain(|&x| x != h);
        }
        self.lists[category.min(ACTORCAT_MAX - 1)].insert(0, h);
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

/// `DynaPoly_GetActor`: the actor that owns bg actor `bg` (its `ActorImpl::dyna_bg_id`), if `bg`
/// is a bg actor in use and not marked for deletion. An actor out of the arena while it updates
/// isn't found (it never looks itself up).
pub fn dyna_poly_get_actor(actors: &ActorContext, col: &eng_collision::bgcheck::CollisionContext, bg: u16) -> Option<ActorHandle> {
    if !col.dyna.is_bg_actor(bg) {
        return None;
    }
    actors.all().into_iter().find(|&h| actors.get(h).is_some_and(|a| a.dyna_bg_id() == Some(bg)))
}

/// `Actor_SetPlayerKnockback`: an actor knocks Player back. Player takes it in its next update
/// (`func_808382DC`): `kind` 1 a push, 2 a knockdown, 3 a shock, at `speed` along `yaw` with
/// `vy` upwards, `damage` added to what the frame's collisions did.
pub fn actor_set_player_knockback(play: &mut PlayState, speed: f32, yaw: i16, vy: f32, kind: u8, damage: u8) {
    if let Some(pi) = play.player.and_then(|h| play.actors.get_mut(h)).and_then(|p| p.as_player_mut()) {
        pi.set_knockback(damage, kind, yaw, speed, vy);
    }
}

/// `Actor_SetPlayerKnockbackLarge`: a knockdown (kind 2).
pub fn actor_set_player_knockback_large(play: &mut PlayState, speed: f32, yaw: i16, vy: f32, damage: u8) {
    actor_set_player_knockback(play, speed, yaw, vy, 2, damage);
}

/// `Actor_SetPlayerKnockbackLargeNoDamage`: a knockdown with no damage of its own.
pub fn actor_set_player_knockback_large_no_damage(play: &mut PlayState, speed: f32, yaw: i16, vy: f32) {
    actor_set_player_knockback_large(play, speed, yaw, vy, 0);
}

/// `Actor_SetPlayerKnockbackSmall`: a push (kind 1).
pub fn actor_set_player_knockback_small(play: &mut PlayState, speed: f32, yaw: i16, vy: f32, damage: u8) {
    actor_set_player_knockback(play, speed, yaw, vy, 1, damage);
}

/// `Actor_SetPlayerKnockbackSmallNoDamage`: a push with no damage of its own.
pub fn actor_set_player_knockback_small_no_damage(play: &mut PlayState, speed: f32, yaw: i16, vy: f32) {
    actor_set_player_knockback_small(play, speed, yaw, vy, 0);
}

/// Where the updating actor's sounds are: `&this->actor.projectedPos` (`play.cur_actor`;
/// `gSfxDefaultPos` outside an actor's turn).
pub fn cur_sfx_pos(play: &PlayState) -> crate::audio::sfx::SfxPos {
    play.cur_actor.map(crate::audio::sfx::SfxPos::Actor).unwrap_or(crate::audio::sfx::SfxPos::Default)
}

/// `Player_PlaySfx`: `Audio_PlaySfxGeneral` at the actor's `projectedPos`, with the defaults.
pub fn player_play_sfx(play: &mut PlayState, actor: ActorHandle, sfx_id: u16) {
    use crate::audio::sfx::{SfxF32, SfxPos, SfxS8};
    play.audio.play_sfx_general(sfx_id, SfxPos::Actor(actor), 4, SfxF32::One, SfxF32::One, SfxS8::Zero);
}

/// `Enemy_StartFinishingBlow`: the frame stops for a flash (`actorCtx.freezeFlashTimer` 5) and the
/// last hit's sound plays where the enemy is, for 20 frames (`SfxSource_PlaySfxAtFixedWorldPos`).
pub fn enemy_start_finishing_blow(play: &mut PlayState, actor: &Actor) {
    play.actors.freeze_flash_timer = 5;
    play.sfx_source_play_sfx_at_fixed_world_pos(actor.world_pos, 20, crate::audio::sfx::NA_SE_EN_LAST_DAMAGE);
}

/// `func_8002DDF4`: Player's `PLAYER_STATE2_12` (holding still on a ladder or a climbable
/// wall: `Player_Action_8084BF1C`).
pub fn func_8002ddf4(play: &PlayState) -> bool {
    /// `PLAYER_STATE2_12` (`player.h`).
    const PLAYER_STATE2_12: u32 = 1 << 12;
    play.player.and_then(|h| play.actors.get(h)).and_then(|p| p.as_player()).is_some_and(|p| p.state_flags2() & PLAYER_STATE2_12 != 0)
}

/// `Math_SinF` and `Math_CosF` (`sys_math.c`): the angle in radians as a binary angle
/// (`(s16)(angle * (0x7FFF / M_PI))`, in double precision) through `sins` and `coss`.
fn math_sin_f(angle: f32) -> f32 {
    eng_math::sin_s((angle as f64 * (32767.0 / std::f64::consts::PI)) as i32 as i16)
}
fn math_cos_f(angle: f32) -> f32 {
    eng_math::cos_s((angle as f64 * (32767.0 / std::f64::consts::PI)) as i32 as i16)
}

/// `Actor_SpawnFloorDustRing`: `amount_minus_one + 1` clouds of brown dust on the actor's floor
/// height in a ring of `radius` round `pos_xz`, drifting up (a random 0.3 ± 0.1) and out at
/// random up to `rand_accel_weight / 2`: `func_8002857C` with `scale` 0, else lit
/// (`func_800286CC`) or not (`func_8002865C`).
#[allow(clippy::too_many_arguments)]
pub fn actor_spawn_floor_dust_ring(play: &mut PlayState, actor: &Actor, pos_xz: Vec3, radius: f32, amount_minus_one: i32, rand_accel_weight: f32, scale: i16, scale_step: i16, use_lighting: bool) {
    let velocity = Vec3::ZERO;
    let mut accel = Vec3::new(0.0, 0.3, 0.0);
    let floor_height = actor.floor_height;
    play.with_ss(|ss| {
        let mut angle = (ss.rand.zero_one() - 0.5) * (2.0 * 3.14);
        let mut pos = Vec3::new(0.0, floor_height, 0.0);
        accel.y += (ss.rand.zero_one() - 0.5) * 0.2;
        let mut i = amount_minus_one;
        while i >= 0 {
            pos.x = pos_xz.x + math_sin_f(angle) * radius;
            pos.z = pos_xz.z + math_cos_f(angle) * radius;
            accel.x = (ss.rand.zero_one() - 0.5) * rand_accel_weight;
            accel.z = (ss.rand.zero_one() - 0.5) * rand_accel_weight;
            if scale == 0 {
                ss.func_8002857c(pos, velocity, accel);
            } else if use_lighting {
                ss.func_800286cc(pos, velocity, accel, scale, scale_step);
            } else {
                ss.func_8002865c(pos, velocity, accel, scale, scale_step);
            }
            angle += (2.0 * 3.14) / (amount_minus_one as f32 + 1.0);
            i -= 1;
        }
    });
}

/// `func_80033480`: `amount_minus_one + 1` puffs of brown dust in a cube `rand_range_diameter`
/// across round `pos_base`, each `scale_base` to 1.2 times it (truncated), rising at 0.3: lit
/// (`func_800286CC`) when `arg6` isn't 0, else not (`func_8002865C`). Four `Rand` calls a puff.
#[allow(clippy::too_many_arguments)]
pub fn func_80033480(play: &mut PlayState, pos_base: Vec3, rand_range_diameter: f32, amount_minus_one: i32, scale_base: i16, scale_step: i16, arg6: u8) {
    let velocity = Vec3::ZERO;
    let accel = Vec3::new(0.0, 0.3, 0.0);
    play.with_ss(|ss| {
        let mut i = amount_minus_one;
        while i >= 0 {
            let x = pos_base.x + ((ss.rand.zero_one() - 0.5) * rand_range_diameter);
            let y = pos_base.y + ((ss.rand.zero_one() - 0.5) * rand_range_diameter);
            let z = pos_base.z + ((ss.rand.zero_one() - 0.5) * rand_range_diameter);
            let pos = Vec3::new(x, y, z);
            // (s16)((scaleBase * Rand_ZeroOne()) * 0.2f) + scaleBase.
            let scale = (((scale_base as f32 * ss.rand.zero_one()) * 0.2) as i16).wrapping_add(scale_base);
            if arg6 as u32 != 0 {
                ss.func_800286cc(pos, velocity, accel, scale, scale_step);
            } else {
                ss.func_8002865c(pos, velocity, accel, scale, scale_step);
            }
            i -= 1;
        }
    });
}

/// `Actor_GetCollidedExplosive`: the actor whose hit the collider took, if it's an explosive
/// (`ACTORCAT_EXPLOSIVE`), the hit cleared (`AC_HIT`).
pub fn actor_get_collided_explosive(play: &PlayState, collider: &mut crate::collision_check::ColliderBase) -> Option<ActorHandle> {
    if collider.ac_flags & crate::collision_check::AC_HIT != 0
        && let Some(h) = collider.ac
        && play.actors.actor(h).is_some_and(|a| a.category == ACTORCAT_EXPLOSIVE)
    {
        collider.ac_flags &= !crate::collision_check::AC_HIT;
        return Some(h);
    }
    None
}

/// `func_80033684`: an exploding explosive (`ACTORCAT_EXPLOSIVE` with params 1: a bomb's
/// explosion) whose blast reaches `explosive_actor` (within `shape.rot.z × 10 + 80`, the
/// explosion's radius growing with its `rot.z`), from the list's head; `me` is
/// `explosive_actor`'s own handle, skipped.
pub fn func_80033684(play: &PlayState, me: Option<ActorHandle>, explosive_actor: &Actor) -> Option<ActorHandle> {
    for &h in play.actors.category(ACTORCAT_EXPLOSIVE) {
        if Some(h) == me {
            continue;
        }
        let Some(actor) = play.actors.actor(h) else { continue };
        if actor.params != 1 {
            continue;
        }
        // Actor_WorldDistXYZToActor(explosiveActor, actor).
        let d = actor.world_pos - explosive_actor.world_pos;
        let dist = (d.x * d.x + d.y * d.y + d.z * d.z).sqrt();
        if dist <= (actor.shape_rot.z as i32 * 10) as f32 + 80.0 {
            return Some(h);
        }
    }
    None
}

/// `Actor_PlaySfx` on the actor `h` (an effect's spawn, for the actor it's spawned for).
pub fn audio_play_actor_sfx_at(play: &mut PlayState, h: ActorHandle, sfx_id: u16) {
    play.audio.play_sfx_at_pos(crate::audio::sfx::SfxPos::Actor(h), sfx_id);
}

/// `Actor_PlaySfx`: `Sfx_PlaySfxAtPos` at the updating actor's `projectedPos`.
pub fn audio_play_actor_sfx2(play: &mut PlayState, sfx_id: u16) {
    let pos = cur_sfx_pos(play);
    play.audio.play_sfx_at_pos(pos, sfx_id);
}

/// `Actor_PlaySfx_SurfaceBomb`: a bounce: `NA_SE_EV_BOMB_BOUND`, then the floor's footstep (the water's
/// when the actor is in water: shallow under 20) at the updating actor.
pub fn actor_play_sfx_surface_bomb(play: &mut PlayState, actor: &Actor) {
    use crate::actor::BGCHECKFLAG_WATER;
    use crate::audio::sfx::{NA_SE_EV_BOMB_BOUND, NA_SE_PL_WALK_WATER0, NA_SE_PL_WALK_WATER1, SFX_FLAG};
    use crate::surface::SurfaceType;
    let sfx_id = if actor.bg_check_flags & BGCHECKFLAG_WATER != 0 {
        if actor.y_dist_to_water < 20.0 { NA_SE_PL_WALK_WATER0 - SFX_FLAG } else { NA_SE_PL_WALK_WATER1 - SFX_FLAG }
    } else {
        // SurfaceType_GetSfxOffset (the C reads through a NULL floor: type 0 here).
        play.audio.tables.surface_sfx_id(actor.floor_poly.map(|p| play.col.sfx_type(p)).unwrap_or(0))
    };
    let pos = cur_sfx_pos(play);
    play.audio.play_sfx_at_pos(pos, NA_SE_EV_BOMB_BOUND);
    play.audio.play_sfx_at_pos(pos, sfx_id.wrapping_add(SFX_FLAG));
}

// `DoorLockType`.
pub const DOORLOCK_NORMAL: usize = 0;
pub const DOORLOCK_BOSS: usize = 1;
pub const DOORLOCK_NORMAL_SPIRIT: usize = 2;

/// `DoorLockInfo`.
struct DoorLockInfo {
    chain_angle: f32,
    chain_length: f32,
    y_shift: f32,
    chains_scale: f32,
    chains_rot_z_init: f32,
    /// `chainDL`, `lockDL`: `(file, symbol)`.
    chain_dl: (&'static str, &'static str),
    lock_dl: (&'static str, &'static str),
}

/// `sDoorLocksInfo`, by `DoorLockType`.
const S_DOOR_LOCKS_INFO: [DoorLockInfo; 3] = [
    DoorLockInfo {
        chain_angle: 0.54,
        chain_length: 6000.0,
        y_shift: 5000.0,
        chains_scale: 1.0,
        chains_rot_z_init: 0.0,
        chain_dl: ("gameplay_dangeon_keep", "gDoorChainDL"),
        lock_dl: ("gameplay_dangeon_keep", "gDoorLockDL"),
    },
    DoorLockInfo {
        chain_angle: 0.644,
        chain_length: 12000.0,
        y_shift: 8000.0,
        chains_scale: 1.0,
        chains_rot_z_init: 0.0,
        chain_dl: ("object_bdoor", "gBossDoorChainDL"),
        lock_dl: ("object_bdoor", "gBossDoorLockDL"),
    },
    DoorLockInfo {
        chain_angle: 0.64000005,
        chain_length: 8500.0,
        y_shift: 8000.0,
        chains_scale: 1.75,
        chains_rot_z_init: 0.1,
        chain_dl: ("gameplay_dangeon_keep", "gDoorChainDL"),
        lock_dl: ("gameplay_dangeon_keep", "gDoorLockDL"),
    },
];

/// `Actor_DrawDoorLock`: a locked door's four chains and its lock, of `DoorLockType` `ty`,
/// under `base` (the door's matrix as the caller left it). `frame` runs from 10 (shut) to 0
/// (open): the chains slide out and the lock shrinks.
pub fn actor_draw_door_lock(out: &mut DrawOut, base: &crate::sys_matrix::MtxF, frame: i32, ty: usize) {
    let entry = &S_DOOR_LOCKS_INFO[ty];
    let mut chain_rot_z = entry.chains_rot_z_init;
    let mut base_mtx = *base;
    base_mtx.translate(0.0, entry.y_shift, 500.0);
    let chains_translate_x = -((10 - frame) as f32) * (entry.chain_angle - chain_rot_z).sin() * 0.1 * entry.chain_length;
    let chains_translate_y = (10 - frame) as f32 * (entry.chain_angle - chain_rot_z).cos() * 0.1 * entry.chain_length;
    let mesh = |(file, symbol): (&str, &str)| eng_gfx::MeshKey::named(crate::pack::keys::mesh(file, symbol));
    for i in 0..4 {
        let mut m = base_mtx;
        m.rotate_z(chain_rot_z);
        m.translate(chains_translate_x, chains_translate_y, 0.0);
        if entry.chains_scale != 1.0 {
            m.scale(entry.chains_scale, entry.chains_scale, entry.chains_scale);
        }
        out.opa.push(eng_gfx::DrawCmd::new(mesh(entry.chain_dl), m.to_mat4()));
        let rot_z_step = if i % 2 != 0 { 2.0 * entry.chain_angle } else { std::f32::consts::PI - 2.0 * entry.chain_angle };
        chain_rot_z += rot_z_step;
    }
    let scale = frame as f32 * 0.1;
    let mut m = base_mtx;
    m.scale(scale, scale, scale);
    out.opa.push(eng_gfx::DrawCmd::new(mesh(entry.lock_dl), m.to_mat4()));
}
