# 0007: Actor ownership: a generational arena, taken out during update

- **Status:** accepted, built in GAME-01 milestone 3
- **Date:** 2026-09-27

## Context

The decomp's actors are linked lists of structs that any code can reach into:
- Player reads the target's position.
- A platform moves whatever stands on it.
- `Actor_UpdateAll` walks every category while actors spawn and kill others.

In Rust, an actor's update needs `&mut self` and access to the rest of the play state, including other actors, at the same time.

## Decision

- **A generational arena** (`oot_game::actor_ctx::ActorContext`): an `ActorHandle` is a slot index plus the slot's generation. A handle to a deleted actor never reaches the slot's next occupant.
- **Categories:** each `ACTORCAT_*` is a list of handles, newest first, as `Actor_AddToCategory` inserts at the head. `Actor_UpdateAll` visits the categories in order, each newest first.
- **Actors embed the base `Actor`**, as the decomp's actor structs start with one.
  - An actor type implements `ActorImpl`: `base` / `base_mut`, the `ActorInit` functions (`update`, `draw`, `destroy`), its share of `AnimationContext_Update`, its draw-time state changes, and its render state.
  - `ActorProfile` holds the `ActorInit` data: id, category, flags and object.
  - Player's 3,600 lines use `self.actor` as before.
- **Updating:** an actor's box is taken out of its slot (`take`) for its update. It gets `&mut PlayState` and reaches any other actor by handle (`actors.actor_mut(h)`, `downcast_mut::<T>(h)`), then goes back (`put_back`).
  - Looking itself up meanwhile finds nothing, though its handle stays valid (`exists`). That matches the decomp: an actor never reaches itself through the lists.
- **Lifecycle:**
  - Spawning (`PlayState::spawn`) inserts a constructed and initialised actor. `Actor_Init` runs the init at once when the object is loaded, and every object is in the pack.
  - Actors spawned during `Actor_UpdateAll` go to the head of their list, so they wait for the next frame.
  - `Actor::kill` is `Actor_Kill`. The actor is deleted (`destroy`, then the slot freed) when `Actor_UpdateAll` reaches it. `isDrawn` isn't tracked, so that's the same frame, not the next.
- **Player** is found through `PlayState::player` and the `PlayerIface` trait: what the framework reads from Player (its age, target, Z timer, stick direction, focus and speed). The content crate adds typed access (`oot_actors::PlayExt::player()`).
- No `RefCell` around actors and no `unsafe`.

## Consequences

- **Cross-actor access** is a lookup by handle, not a reference held across the update: Player's target, `unk_664`, is an `Option<ActorHandle>`.
- **Frame order.** Framework code runs in the decomp's frame order around the actor updates (`oot_game::play`): the target context, DynaPoly, `AnimationContext_Update`, the cameras, the draw-time state.
- **What isn't modelled yet:**
  - culling: `ACTOR_FLAG_6` is set for every actor, so every actor counts as in view;
  - `isDrawn`;
  - the `ACTOR_NUMBER_MAX` overlay-memory rules beyond the count;
  - object loading (`Object_IsLoaded`), since every object is in the pack.

## Amendment (milestone 4)

- **Spawning by id** is `PlayState::actor_spawn` (`Actor_Spawn`): the profile comes from the pack's actor table, and the constructor from `oot_actors::overlays()` or `Placeholder` (ADR 0010).
- **Init waits for the object:**
  - An actor whose object is still loading sits in its slot as `oot_game::spawn::Uninit` (its base `Actor` and its constructor).
  - `Actor_UpdateAll` swaps the constructed actor in once the object is loaded, keeping the handle.
  - Constructors run with `&mut PlayState`, so an actor's init can spawn others (Player spawns Navi). Those go to the head of their list before the constructed actor is inserted, where the decomp inserts the parent first.
- **Actor fields:** `Actor::room` is the real room (-1 for none), and actors spawned by id carry `obj_bank_index`.
- **Player's writes to the play state** during its update go through `PlayIo` (ADR 0010).
