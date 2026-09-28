# 0016: Player's requests to the play state, and actors writing each other

- **Status:** accepted, built in GAME-02 milestone 2; extends ADR 0007 and ADR 0010's `PlayIo`
- **Date:** 2026-09-28

## Context

Opening a door is a conversation between two actors and the play state:

- **`EnDoor_Idle`** (category `ACTORCAT_DOOR`, after Player) writes into Player:
  `doorType`, `doorDirection`, `doorActor`.
- **Player's `func_80839800`**, next frame, reads them, then does four things in place:
  - writes into the door: `openAnim`, `playerIsOpening`, its `room` and its double's;
  - calls the camera: `Camera_ChangeDoorCam`, `Camera_ChangeSetting` through
    `func_80835E44`;
  - loads a room: `func_8009728C`;
  - later, drops the old room (`func_80097534`) and tells the camera it's through
    (`func_8005B1A4`).

ADR 0007 takes the updating actor out of the arena, so Player can't reach the camera, the
room context or the door while it updates. ADR 0010's `PlayIo` lends out the save and the
transition fields, but not the camera, the rooms or other actors.

## Decision

- **Player queues what it does to the rest of play** (`oot_actors::player::PlayRequest`):
  - camera setting changes, the door camera, `func_8005B1A4`;
  - room loads and the room change;
  - the door's opening and its room.

  Its `ActorImpl::update` applies them in order, right after `Player::update` and before
  `Player_UpdateCamAndSeqModes`' mode request. That's the order the C makes them in.
- **What Player reads of the camera is last frame's.** For example `unk_14C & 0x10` in
  `func_80845CA4` is passed in (`Env::cam_unk_14c`). That's what the C reads too, since
  `Camera_Update` runs after the actors.
- **An actor that writes another does so through the arena, downcasting**:
  - `EnDoor` writes Player's `door_type`, `door_direction` and `door_actor` with
    `play.actors.downcast_mut::<Player>` (Player is back in the arena by then);
  - Player's queued `OpenDoor` writes the `EnDoor`.

  Both types are in `oot_actors`, so no trait in `oot_game` needs to know about doors.

## Consequences

- **The order within one Player update is kept for everything queued**, and nothing Player
  reads later in the same update depends on it:
  - the C's door code reads nothing back from the camera or the rooms after changing them;
  - `func_80839034` (an exit under a scene-exit door) writes only `PlayIo`.
- **A request only takes effect after Player's update.** Anything that would have to see it
  within the update needs another way. None does so far.
- **More cross-actor writes will follow this pattern:** talking (`Actor_ProcessTalkRequest`),
  held actors, `Door_Shutter`. If `PlayRequest` grows unwieldy, it can move to `oot_game`
  with a generic "apply to actor" variant.
