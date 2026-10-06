# 0038: Sliding doors and room travel

- **Status:** accepted, built in GAME-05 milestone 4a (2026-10-06)
- **Date:** 2026-10-06
- **Builds on:** [ADR 0007](0007-actor-ownership.md) (the arena, overlay statics),
  [ADR 0010](0010-scenes-and-spawning.md) (rooms, transition actors),
  [ADR 0015](0015-camera-settings-and-bg-cameras.md) (the door camera),
  [ADR 0029](0029-one-point-cutscenes.md) (attention cameras) and
  [ADR 0037](0037-the-deku-trees-other-enemies.md) (debug starts).

## Context

Milestone 4 makes the Deku Tree's rooms reachable without debug starts. Its first part, 4a,
brings the dungeons' sliding doors (`Door_Shutter`, nine in MQ's Deku Tree) and what moving
between rooms needs. The doors are transition actors like `En_Door`, but they slide instead of
swinging, and several are barred until their room is cleared or a switch is set.

Seven things needed a decision:
- **Player's side.** `Player_ActionHandler_1`'s `PLAYER_DOORTYPE_SLIDING` branch only logged. It
  walks Link through the door with `Player_Action_80845CA4`, the exits' walk, already ported.
- **The doorway's wall.** A closed shutter has no collision of its own. The scene's mesh has a
  wall in each doorway, and Link stopped against it.
- **The room's clear.** A door barred until the room is cleared (`SHUTTER_FRONT_CLEAR`) reads the
  room's temporary clear flag. Nothing set it: the hint scrubs set the permanent one themselves.
- **"On top" flags.** The floor switch and the floor web read `DynaPolyActor_IsPlayerOnTop` and its
  kin, which weren't ported.
- **Small keys.** No door or chest in MQ's Deku Tree takes or gives one, but `Door_Shutter`'s
  key-locked type and the HUD's key counter are part of the scope.
- **The lock's chains.** `Actor_DrawDoorLock`, which `En_Door` had left out.
- **The debug starts.** Every room needs one, checked.

## Decision

- **`Door_Shutter` is ported whole** (`oot_actors::door_shutter`): every type and style, the
  bars, the slam, the Jabu Jabu door's sections, the boss door's texture per dungeon (a bake
  each), the Gohma slab and Phantom Ganon's bars with their collision. The Gohma slab's quake
  takes `Boss_Goma`'s sub camera, which isn't ported: the main camera stands in (logged). Rumble
  isn't ported.
- **Player's sliding door** is the C's branch: the yaw from the door's `home.rot.y`, the walk 20 to
  the door then 120 past it (`unk_450`, `unk_45C`), `isActive` set on the door
  (`PlayRequest::SlidingDoorActive`), `cv.slidingDoorBgCamIndex` kept for the door's
  `Camera_ChangeDoorCam`, a locked door's `doorTimer` wait, and the shared tail (the room behind
  loaded, the door's room). The door's `DoorShutter_SetupClosed` swaps the rooms back if Link
  went back, finishes the room change and sets the respawn point; a door barred behind Link holds
  him (`PLAYER_CSACTION_2`, then 7 after 32 frames).
- **`Player_ProcessSceneCollision` picks its bg check flags as the C does.** During a door's walk
  (`Player_Action_80845EF8`, `Player_Action_80845CA4` without `PLAYER_STATE1_0`) it drops the wall
  check, so Link walks through the doorway's wall; `PLAYER_STATE1_31` drops the floor; an
  entrance walk after a fall of 100 keeps only the wall. The port had always passed all six flags.
- **`Actor_RemoveFromCategory` sets the room's temporary clear flag** when the current room's last
  enemy goes (`PlayState::actor_remove_from_category`, used by both deletions). With it come
  `Flags_SetTempClear`, `Flags_UnsetTempClear` and `Flags_UnsetSwitch`.
- **DynaPolyActor's interact flags live on the engine's bg actor slot** (`BgActor::interact_flags`,
  `DYNA_INTERACT_*`), in a `Cell`: the C sets them from code that only reads the collision
  (`Actor_UpdateBgCheckInfo`'s `func_80043334`, Player's `DynaPoly_SetPlayerOnTop` and
  `DynaPoly_SetPlayerAbove`). An actor that reads them returns its bg id from
  `ActorImpl::dyna_bg_id`, and `Actor_UpdateAll` clears them after its update
  (`DynaPoly_UnsetAllInteractFlags`).
- **Small keys:** the key-locked door spends one from `dungeonKeys[mapIndex]` (tested on a door the
  test makes key-locked), and `Interface_Draw`'s key icon and counter are drawn in the scenes its
  `switch` lists, which leave out the first three dungeons.
- **`Actor_DrawDoorLock`** is ported in `oot_game::actor_ctx` (the chains and the lock from the
  pack's meshes, under the caller's matrix), and both doors draw it.
- **Debug starts:** `playthrough::DEKU_TREE_ROOM_STARTS` has one per room but room 11 (its whole
  floor is exit 2, the drop into Gohma's room), plus one on room 0's top floor. `oot_actors --test
  debug_starts` places Link at each, with the room's enemies out of the way, and runs 40 frames:
  he stands where he was put, in that room. The game's and the sandbox's `--room`/`--at` take the
  same values (`game-dungeon.bat`).
- **The exit's run** (`Route::Shutter`): from room 0's top floor, the floor switch, the web it
  burns, room 10's door opened and walked through, and the door barred behind.

## Consequences

- From room 0, Link reaches room 10 (through its door, once the switch has burnt the web), room 3
  (through the floor web, by falling on it from the top floor), and from room 3 room 9 and then
  11 once the floor web there burns (milestone 4b's Deku Stick). Rooms 2 and 4 to 8 wait for the
  slingshot's eye switches (milestone 5).
- A cleared room unbars its doors with their attention cameras, which hold the actors while they
  run. The hint scrubs' test waits longer for the last two scrubs to sink, since solving the
  puzzle clears room 9.
- Door walks no longer test walls: an `En_Door`'s opening animation in a scripted run can no
  longer be stopped by a wall (none was).
- The Gohma slab's and the bars' parent (`Boss_Goma`, `Boss_Ganondrof`) aren't ported; the slab is
  tested on its own.
