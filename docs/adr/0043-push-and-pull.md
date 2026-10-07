# 0043: Push and pull

- **Status:** accepted, built in GAME-05 milestone 4c (2026-10-07)
- **Date:** 2026-10-07
- **Builds on:** [ADR 0007](0007-actor-ownership.md) (the arena, Player out of it while he
  updates), [ADR 0037](0037-the-deku-trees-other-enemies.md) (draw-time state in `draw_update`),
  [ADR 0038](0038-sliding-doors-and-room-travel.md) (the interact flags on the engine's bg actor)
  and [ADR 0041](0041-the-deku-stick-and-the-item-buttons.md) (branches for what isn't ported log).

## Context

Milestone 4c brings Player's push and pull, for room 3's push block (`Obj_Oshihiki`, which
`Obj_Makeoshihiki` spawns) and room 7's gravestones (`Bg_Haka`). `Player_ActionHandler_5` showed
"Grab" at a pushable wall and logged on A.

The C passes the push between Player and the block through the block's `DynaPolyActor`: Player
adds his force to `unk_150` and sets `unk_158` to his yaw (`z_actor.c`'s `func_8002DFA4`), the
block reads them in its own update, and clears Player's `PLAYER_STATE2_4` when it stops taking
the push. Five things needed a decision:
- **Where `unk_150`, `unk_154` and `unk_158` live.** Player writes them during his update, when he
  only reads the collision context and is out of the actor arena.
- **`DynaPoly_GetActor`.** Player keeps the wall's actor (`unk_3C4`) and compares it each frame;
  the block asks whether the floor under it is another push block or a floor switch. The engine's
  bg actor slots don't know their owners.
- **An actor's own collision in its tests.** The block looks for floors and walls past its own
  collision (`BgCheck_EntityRaycastDown6`, `BgCheck_EntityLineTest3`).
- **The heavy block's branch.** `Player_ActionHandler_5` lifts a `Bg_Heavy_Block` with the gold
  gauntlets (`func_8083A0F4`), and Player's lift isn't ported.
- **A block riding on another.** `ObjOshihiki_MoveWithBlockUnder` moves the block from its draw,
  and `Obj_Makeoshihiki`'s draw sets and clears its flags.

## Decision

- **Player's push and pull are ported whole:** `Player_ActionHandler_5`'s push branch,
  `func_8083F72C`, `func_8083F9D0`, `func_8083FAB8`, `func_8083FB14`, `func_8083FFB8`,
  `func_8083F524`, `func_8084B840`, the actions `Player_Action_8084B78C` (holding on),
  `_8084B898` (pushing) and `_8084B9E4` (pulling) with their sounds (`NA_SE_VO_LI_PUSH`, the
  floor's slips), `func_8083A388` as the put-away's next action, `PLAYER_STATE2_4` (and
  `func_80832440` clearing it), and `Player_GetStrength` (`oot_game::player_lib`).
  `CAM_MODE_PUSH_PULL` was already requested for `PLAYER_STATE2_8` and runs `Camera_Parallel1`.
- **The force lives on the engine's bg actor slot,** next to the interact flags: `BgActor::unk_150`,
  `unk_154` and `unk_158`, in `Cell`s, with `Dyna::func_8002DFA4` (only for a bg id in use, as
  `DynaPoly_GetActor` returns NULL otherwise), `func_8002DF90`, and getters and setters for the
  owner. `DynaPolyActor_Init` zeroes them, as a new slot does.
- **`DynaPoly_GetActor` is `oot_game::actor_ctx::dyna_poly_get_actor`:** the actor in the arena
  whose `ActorImpl::dyna_bg_id` is the bg id, for a bg id in use. Every DynaPoly actor now returns
  its bg id (`Bg_Spot00_Hanebasi`, `Bg_Treemouth` and `En_Box` didn't), which also clears their
  interact flags after their update, as the C's `Actor_UpdateAll` does for every actor; nothing
  reads those three's flags. An actor out of the arena (one updating, or Player) isn't found; none
  looks itself up.
- **`CollisionContext::entity_raycast_down6` and `entity_line_test3`** skip the caller's own bg
  actor (`check_line_skip`, as `raycast_down_skip` does for the floors).
- **The heavy block's branch is ported to its checks** (the user's choice): the actor id test and
  `Player_GetStrength() < PLAYER_STR_GOLD_G`; past them it logs that the lift isn't ported and
  returns as the C does, as ADR 0041 did for the items no button holds.
- **`Obj_Oshihiki` and `Obj_Makeoshihiki` are ported whole,** their draws' state in `draw_update`
  (ADR 0037): the block's ride on the one under it (and the draw's `Matrix_Translate` that makes
  up for the frame, drawn from where it was), the maker's flags and chime. The block's
  `gPushBlockDL` is baked once per texture with the env colour dynamic (pack format 22). Bits the
  game never places are kept as the C has them, `@bug (game)`: an unknown type's `bgId` is the
  zeroed actor's 0 and its scale is read from past the table; outside the scenes the debug draw
  lists, the colour is `mREG(13..15)`, 0 here.

## Consequences

- Room 3's block can be pushed along its channel and off its end into the pit, where it's the
  step back up to the upper floor (`Route::Push`, the golden `push`). Player's pull moves a
  gravestone (`Bg_Haka`) too, though Master Quest's are sunk too low to hold (ADR 0044).
- The first frame Player starts pushing, the block refuses the push (Player adds to `unk_150` from
  the action's next frame), clearing `PLAYER_STATE2_4`: a frame later the push is taken. That's
  the C's order (the block, `ACTORCAT_PROP`, updates after Player).
- **Order of an init's children:** the C adds an actor to its category before its init runs, so
  children an init spawns come before their parent in the lists; the port inserts an actor after
  its init, so its children come after it. For `Obj_Makeoshihiki` this changes nothing (its update
  is a no-op, and only a block riding on another reads the draw order). It's in the backlog.
