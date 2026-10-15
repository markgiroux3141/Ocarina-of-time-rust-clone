# 0071: The HUD's timers, and Talon's house: the milk and the Super Cucco game

- **Status:** accepted, built in GAME-08 milestone 3, step (b) (2026-10-15)
- **Date:** 2026-10-15
- **Builds on:** [ADR 0070](0070-lon-lon-ranch-the-horses-ingo-and-the-guays.md) (step (a): the
  timers' save fields and `Interface_SetTimer`), [ADR 0060](0060-hyrule-castles-grounds.md)
  (`En_Ta`'s castle, Kakariko and night parts), [ADR 0064](0064-kakariko-lifting-and-the-village.md)
  (lifting the cuccos).

## Context

Step (b) of Phase 9's last milestone (`docs/GAME-08-woods-meadow-and-ranch.md`): Talon's house by
day (BACKLOG #44), which `En_Ta` left as a placeholder in GAME-07. Talon sells milk (30 rupees, an
empty bottle) once he has given his bottle, and runs the Super Cucco game: 10 rupees, his three
super cuccos thrown among the house's ten, 30 seconds to bring each back to him, a full bottle of
milk the first time it's won.

The game runs on the HUD's main timer: `Interface_SetTimer(30)` starts it, and Talon reads
`timerSeconds` (the fast tempo at 10, time's up at 0). Step (a) added the timers' save fields and
`Interface_SetTimer`; nothing counted or drew them. In the C the timers' state machine is part of
`Interface_Draw`, in the draw pass, with function statics (`sTimerNextSecondTimer`,
`sTimerStateTimer`, the sub timer's, `sTimerDigits`) and the file's `sEnvHazardActive`;
`Interface_Update` starts the hot rooms' and the water's timer (`TIMER_STATE_ENV_HAZARD_*`).

The music (`func_800F5ACC`, `func_800F5B58`, `Audio_SetFastTempoForTimedMinigame`) and the super
cuccos' flight (`En_Niw` 0xD's `unk_308`) were already ported.

## Decision

1. **The timers whole** (`oot_game::timer`): `Interface_SetTimer`, `Interface_SetSubTimer`,
   `Interface_SetSubTimerToFinalSecond`; `Interface_Draw`'s state machine for the main timer
   (count down, count up, the hazard's, the stop and the freeze) and the sub timer (the trade
   quest's respawn with its text 0x71B0, the marathon man's limit), the beeps, the digits, and
   the draw: the clock and five digits, red under 10 seconds, the sub timer yellow. The state
   machine runs in the game frame where `Play_Draw` would run `Interface_Draw` (after the reticle,
   before the message box), as the effects' draw-time state does (ADR 0033); `TimerStatics::draw`
   draws what it left. The statics live in the interface context and `PlayState::reinit` carries
   them over, as they outlive the play state in the C.
2. **`Interface_Update`'s hazard timer:** started in a hot room or under water while Link has
   health to lose, stopped out of the hazard; running out kills Link through `play->damagePlayer`,
   which the port now has as the C does: a hook on the play state that `Player_Init` sets
   (`PlayState::damage_player`, `Player_InflictDamage`).
3. **The callers brought up to date:** Player's item buttons wait out a just-stopped timer
   (`TIMER_STATE_STOP`); `Bg_Relay_Objects` freezes the race's timer; `Demo_Kankyo` calls
   `Interface_SetSubTimerToFinalSecond`; Ingo's rental calls `Interface_SetTimer(60)`.
4. **`En_Ta`'s ranch house whole** (a helper), into the existing module: the day's init (the three
   super cuccos), the talks and their choices, the milk, the game (the throw, the hunt, the 7-frame
   hold, time's up with the floor camera, the starburst back to `ENTR_LON_LON_BUILDINGS_2`), the
   prizes after it, its destroy (the timer off, the music back). BACKLOG #44 closed.
5. **The run (`Route::Ranch`, step (a)'s with more):** after the foal, back out of the corral and
   up the gate's lane to the house's door (`Task::OpenDoor`: the first route through an `En_Door`),
   Talon's 0x207E answered yes, the three super cuccos caught (`Task::CatchSuperCucco`: the
   nearest one Talon still counts, by its gravity), back in the house, his 0x2086 and 0x2087 and
   the bottle of milk.

## Consequences

- The timers are ready for the games and races to come: the horseback archery and Ingo's race
  (Phase 13), the trade quest's spoiling items (#39's masks' phase onwards), the marathon man,
  Dampé's race (Phase 14), the hot rooms and the water (Phases 10 and 11). `minigameState` and
  `shootingGalleryStatus` read as 0 until those are ported.
- No golden changed with the timers or the hazard timer: no recorded run dives, enters a hot room
  or starts a timer.
- The digits draw their texels' 9.3 by 18.6 pixels where the C's rectangle is 9 wide (and 250
  tall, its clamped last row blank): a third of a pixel at the right of each digit.
