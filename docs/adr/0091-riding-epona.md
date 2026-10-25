# 0091: Riding Epona

- **Status:** accepted, built in GAME-12 milestone 1 (2026-10-24)
- **Date:** 2026-10-24
- **Builds on:** [ADR 0089](0089-the-door-of-time-to-the-chamber-and-out.md) (`adult-awake`, Phase
  13's start), [ADR 0070](0070-lon-lon-ranch-the-horses-ingo-and-the-guays.md) (the ranch, the foal
  `En_Horse_Link_Child` and Epona's Song), [ADR 0066](0066-the-actor-schema.md) (the actor schema).

## Context

Phase 13 rides Epona: Ingo's rental, the races, Epona won and the jump out, all on horseback. The
horse (`En_Horse`), Player's riding and the horse camera are one system; nothing of it was ported.
Epona's Song set `R_EPONAS_SONG_PLAYED` and nothing read it (BACKLOG #51). The survey found that
the horse's movement uses no `Rand` (the dust does), and that `riderPos`, where Player rides, comes
from her draw (limb 30's post-draw), so the draw's order matters to the update's.

## Decision

1. **`En_Horse` whole**, but for the parts Phase 18's inits choose (horseback archery, the Gerudo
   Valley bridge jump's start, the Gerudo saddle: logged, the horse stands) and rumble. Every
   action is ported, Ingo's race and the cutscene actions with them, for milestones 2 and 3. Its
   post-draw runs in `ActorImpl::draw_update` (once a game frame, in draw order): `riderPos`, the
   head's point, the dust, the jumps (`postDrawFunc`: `EnHorse_LowJumpInit`, `_HighJumpInit`,
   `_BridgeJumpInit`) and the jnt sphere's places.
2. **`z_horse.c` in `oot_game::horse`**, whole: `Horse_InitPlayerHorse` runs in `Play_Init` after
   the scene's actors, as the C. `R_EXITED_SCENE_RIDING_HORSE` (`AREG(6)`) lives in `SaveContext`,
   so the next scene's `Play_Init` sees what the exit set.
3. **Player's riding whole** (action handler 3, the mount, `Player_Action_8084CC98`,
   `Player_Action_8084D3E4`, re-mounting at a scene's start, the exits on horseback). Player is out
   of the arena during his update: he reads the horse through `env.actors` and changes it through
   `PlayRequest`s (`HorseChild`, `HorseClearFlag8`, `HorseCameraSetting`) applied after his update.
   The horse writes Player through `PlayerIface` (`set_ride_actor`, `mount_horse`).
4. **The horse camera:** `Camera_Normal3` (dispatched by `CAM_SET_HORSE`'s mode function, as the
   others) and `Camera_CalcAtForHorse`, reading the horse through `PlayerView::ride` (its place and
   `EN_HORSE_CHECK_JUMPING`, from `ActorImpl::horse_jumping`).
5. **`Bg_Umajump` whole.** The carrots (`gCarrotIconTex`) and the horse's skeletons, animations
   and eyes are baked (pack format 35).
6. **The runs:** `Route::RideEpona` (`adult-awake`, the corral: the song, on her, a fence jumped,
   off her; its end `adult-awake`) and `Route::FieldRide` (the debug preset **`epona`**,
   `adult-awake` with `EVENTCHKINF_EPONA_OBTAINED`: below the ranch's gate, on her, up the ramp and
   into the ranch on her; its end `epona`). The tasks: `Mount` (to her side, at her until "Climb",
   A), `Ride` (through points: the stick from her heading, stopped and turned in place for a point
   far off it, spurs when lined up, halted at the end), `RideExit`, `RideIn`, `Dismount`. They read
   the offer by the "Climb" do-action: Player clears `rideActor` at each update's end.

## Consequences

- BACKLOG #51 closes. #68 (the horses' sound positions) now covers `En_Horse` too: her voice plays
  at the horse, not at her head (`unk_21C`). New: #111, the camera's speed ratio swimming.
- Milestones 2 to 4 build on the horse as it is: Ingo's race and the cutscene actions are there to
  run; `En_In` and `Bg_Ingate` are the next.
- The field run doesn't jump a low wall (the field's adult layer has no `Bg_Umajump`): the field's
  low walls are the hand test's.
