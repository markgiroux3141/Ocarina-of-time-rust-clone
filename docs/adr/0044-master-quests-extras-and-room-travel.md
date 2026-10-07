# 0044: Master Quest's extras and room travel

- **Status:** accepted, built in GAME-05 milestone 4c (2026-10-07)
- **Date:** 2026-10-07
- **Builds on:** [ADR 0038](0038-sliding-doors-and-room-travel.md) (the doors, the debug starts),
  [ADR 0042](0042-the-deku-trees-props-and-the-fragments.md) (triggers injected, the fragments)
  and [ADR 0043](0043-push-and-pull.md) (push and pull).

## Context

Master Quest's Deku Tree places three more kinds of actor than the mechanics milestone 4a and 4b
ported: room 7's gravestones (`Bg_Haka`), the Song of Time's blocks in rooms 2, 5 and 7
(`Obj_Timeblock`), and room 2's rocks (`Obj_Bombiwa`). Link has no ocarina or bombs yet. The
milestone also makes room-to-room travel solid: one test through every connection milestone 4
opened. Four things needed a decision:
- **The song.** `Obj_Timeblock` watches Player's `PLAYER_STATE2_24` (the ocarina out), starts the
  free play (`Message_StartOcarina`), and reads `msgCtx.lastPlayedSong` and `ocarinaMode`. None of
  that is ported, and the song spawns `Demo_Effect` (2,092 lines).
- **Room 7's gravestones** turn out to be sunk 15 into the floor, too low for Player to hold on to
  (`Player_ActionHandler_5` wants a wall 39 high; theirs is 34, 28 where he measures).
- **Room 5's shown time block** stands under room 5's Skulltula, which now finds its floor on the
  block's top and doesn't drop to Link below.
- **How real the travel test's walking is.** Some connections need what Link can't do yet
  (the slingshot's loop back to room 3's upper floor), and room 0's middle floor has no way up
  that the port lets Link climb.

## Decision

- **`Bg_Haka`, `Obj_Timeblock` and `Obj_Bombiwa` are ported whole** (`bg_haka`, `obj_timeblock`,
  `obj_bombiwa`), by a worktree agent in parallel with Player's push and pull. The time block is a
  bake with its prim colour dynamic; the rocks' fragments are one more `Effect_Ss_Kakera` list
  (pack format 22).
- **The song is injected:** `MessageContext::last_played_song` is a new field (`msgCtx.lastPlayedSong`,
  0 as the zeroed play state leaves it); the tests set it and Player's `PLAYER_STATE2_24` as the
  ocarina would. `Message_StartOcarina` logs and the block still waits for the song, as the C does.
  `Demo_Effect` and `En_Poh` (a night-time graveyard Poe) stay placeholders, as agreed.
- **The survey's time-block reading is corrected:** MQ's 0x39FF and 0xB9FF set params bit 6, so
  the block's own state is params bit 15, flipped by the song (`unk_177` 0), not switch flag 0x3F.
- **Room 7's gravestones are left as the data has them:** in Master Quest Link climbs onto them.
  Player's real pull is tested on a stone the test spawns flush with the floor (as the graveyard
  places them), the placed ones by injection.
- **`skulltula_st` hides room 5's time block as the song would** before its Skulltula's tests
  (params bit 15 off, collision off): with the block there, the Skulltula doesn't drop, which is
  the game's behaviour. Its drop test now maps the drop table through `func_8001F404`, as the drop
  does (the block's first frames move the `Rand` stream: a green rupee for a recovery heart at full
  health).
- **The travel test** (`oot_actors --test travel`) runs from one `Play_Init`: 0 to 10 and back, 0
  to 1 and back, the drops 0 to 3 and 3 to 9, then 9 to 11 once the hint scrubs' puzzle (nut hits
  injected in order) clears room 9. Every room change is the real door or drop; within a room Link
  is placed where getting there needs what isn't ported: room 0's top floor (the middle floor's
  vines take him up by hand, but the test's scripted climb didn't), room 3's upper floor (the slingshot's loop through rooms 4
  to 7), and beside the running scrub. Room 10's enemies are killed by injection.

## Consequences

- Room 5 has its block over the purple rupee's chest, solid, and rooms 2 and 7 their hidden blocks
  (no collision) until the ocarina comes. Rooms 2, 5 and 7 invalidate the DynaPoly lookup every
  frame (`DynaPoly_Enable/DisableCollision` each update), as the C does.
- Room 2's rocks are placed turned, so their init draws no `Rand`; one placed with no yaw would.
- Bombs, the hammer and the ocarina will reach these actors through the paths the tests inject.
