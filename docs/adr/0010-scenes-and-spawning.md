# 0010: Scenes, rooms and spawning: `Play_Init` per scene, actor profiles from the C, placeholders

- **Status:** accepted, built in GAME-01 milestone 4
- **Date:** 2026-09-27

## Context

Milestone 3 gave the game a `PlayState` and an actor system, but a scene was still the spikes' view: its collision and every room drawn, Player alone. Phase 2's second half asks for:
- scenes and rooms loaded from the pack;
- room changes;
- spawning from the rooms' actor lists, with a placeholder for every actor that isn't ported;
- transition actors and exits;
- Kokiri Forest loaded with every placement implemented or a placeholder, and Link walking into his house and back out.

The decomp's flow is spread over `z_play.c` (`Play_Init`, `Play_Update`'s transition modes), `z_scene.c` (header commands, the object banks), `z_room.c` (room loads), `z_actor.c` (`Actor_Spawn`, transition actors, room-change kills), the fade (`z_fbdemo_fade.c`) and Player (`Player_HandleExitsAndVoids`, the start modes, `Player_Action_80845CA4`).

Four questions needed answers:
1. How a scene change maps onto a Rust `PlayState`.
2. Where actor profiles come from.
3. What an unported actor becomes.
4. What happens to the decomp's asynchronous loads when everything is in the pack.

## Decision

- **A scene change rebuilds the play state, as the game does.**
  - `PlayState::play_init(assets, data, rules, save)` is `Play_Init`.
  - When a transition ends the scene, `Play_Update` sets `gSaveContext.save.entranceIndex` and the next game state is a fresh `Play_Init`. Here that's `reinit` at the end of the frame: a new `PlayState` from the same `SaveContext`, carrying over the pad, the debug switches and the camera choice.
  - What outlives a scene is `SaveContext` (`oot_game::save`): the parts of `gSaveContext` play reads and writes, namely the entrance, the age, the time, the respawn points, the entrance speed and the next transition type.
- **Actor profiles are data from the C.**
  - The importer reads `actor_table.h` and every `<Name>_Profile` (category, `FLAGS` evaluated, object, function names) into `table/actors` (`oot_game::actor_table`).
  - It keys them by symbol, not by the id field: `Boss_Dodongo_Profile` says `ACTOR_EN_DODONGO`, and four overlays say 0. `Actor_Spawn` copies that field into the actor, so the quirk is kept (`@bug (game)`).
  - A test checks the ported actors' `ActorProfile` constants against the table.
- **Ported actors register a constructor; every other id is a `Placeholder`.**
  - `oot_actors::overlays()` maps `ACTOR_*` ids to `fn(Actor, &mut PlayState) -> Box<dyn ActorImpl>`: the actor's init, given the base `Actor` that `Actor_Spawn` + `Actor_Init` set up.
  - An unported id spawns a `Placeholder` with its real profile: category, flags, object dependency and room. So the category order, the object rules and the room-change kills treat it as the game treats the real actor. It does nothing, and `Debug::placeholders` draws a marker.
- **The loads keep their timing, not their mechanism.**
  - **Rooms:** a room load requested in one frame (`Room_RequestNewRoom`) finishes at the next `Room_ProcessRoomRequest`, the following frame. Then the room's header runs (actor list, object list, behaviour) and the transition actors spawn. A scene's rooms for its layer are read from the pack at `Play_Init`.
  - **Objects:** `Object_LoadPersistent` loads at once (it's a blocking DMA in the game). Objects a room's object list swaps in load one frame after the swap: the first `Object_UpdateEntries` starts the DMA, the next one sees it done.
  - **Init deferral:** an actor whose object is still loading waits as an `Uninit` (`actor->init != NULL`). `Actor_UpdateAll` runs its constructor once the object is loaded, and the actor skips that frame. Actors whose bank is dropped are killed (`Actor_KillAllWithMissingObject`).
- **Player's writes to play go through `PlayIo`.** The decomp's Player writes `play->transitionTrigger`, `nextEntranceIndex`, `gSaveContext.respawn…` from inside its update, while Player is taken out of the actor context (ADR 0007).
  - `PlayState::take_io` lends out the save, the transition state and the scene flags in a `RefCell` for the update, and `put_io` takes them back.
  - `Play_TriggerVoidOut`, `Play_SetupRespawnPoint`, `Scene_SetTransitionForNextEntrance` and friends are methods on `PlayIo`, so Player and the framework share one implementation.
- **Length-less lists are bounded like the decomp's extraction does.**
  - The entrance and exit lists carry no count. The importer ends them at the next offset any header command points at, and only while their entries are plausible (`oot_import::scene::list_extent`).
  - A test checks they cover what gameplay indexes: every exit index a floor uses, and the entrance-list entry each such exit leads to for the traveller's layer. Entrance rows past a list are reached only by cutscenes and some actors, and that one exception is documented (below).

## Consequences

- Kokiri Forest enters with its 78 room-0 placements, Navi, and both `En_Holl` planes. Every scene enters and plays for both ages (220 entries in the sweep test).
- **Exits work anywhere a floor carries one:** Link's house and back is the tested case.
- **Room changes work through `En_Holl`, the only transition actor ported.**
  - Doors (`En_Door`, `Door_Shutter`) are placeholders, so rooms behind doors can't be entered yet.
  - Placeholders don't run their init. So an actor whose init moves it out of every room (`room = -1`: Navi, `Object_Kankyo`, `En_Kusa`…) is deleted by a room change that the real one survives.
- **`@bug (game)` found by the coverage test.**
  - Bongo Bongo's room's exit 1 leads to `ENTR_SHADOW_TEMPLE_2`, spawn 2, past the Shadow Temple's two-entry entrance list; the game would read the exit list's first entry as the entrance entry.
  - `Play_Init` here fails that entrance with an error instead.
- **Transitions:** `TransitionFade` and the white and brown fills are ported frame for frame. The wipe, triforce and circle transitions run as a black fade of the fade's default length, and the sandstorm and cutscene fills end at once.
- The pack's format version is 2: new records (`table/actors`, the entrance table in `table/scenes`) and new fields in `LayerData` and `RoomData`.
