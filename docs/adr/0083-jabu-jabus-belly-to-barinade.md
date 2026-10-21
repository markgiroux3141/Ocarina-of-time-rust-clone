# 0083: Jabu-Jabu's Belly to Barinade: Ruto, the tentacles and barriers, and the way to the boss

- **Status:** accepted, built in GAME-10 milestone 5 (2026-10-21); its hand test owed ([HAND-TESTS-OWED.md](../HAND-TESTS-OWED.md))
- **Date:** 2026-10-21
- **Builds on:** [ADR 0082](0082-jabu-jabus-belly-to-the-boomerang.md) (Jabu-Jabu to the boomerang,
  the search over the floors), [ADR 0064](0064-kakariko-lifting-and-the-village.md) (Player's
  lift, carry and throw), [ADR 0066](0066-the-actor-schema.md) (the schemas),
  [ADR 0079](0079-the-level-texts.md) (the texts).

## Context

Phase 11's fifth milestone (`docs/GAME-10-zoras-waters-and-jabu-jabu.md`): from the boomerang's
chest to Barinade's room. Ruto (`En_Ru1`, one overlay for every place she appears), the tentacles
(`En_Ba`), the electric barriers (`En_Bx`), the small crate (`Obj_Kibako`) and the room timers
(`Obj_Roomtimer`) were placeholders; Player's two Ruto checks were missing; the story's presets
after the diving game lacked the silver scale (BACKLOG #102). The survey left room 2's holes, the
cow's boulders, room 11's stick and room 5's climb to door 0 open.

## Decision

1. **The silver scale in the story's presets** (the user's call on #102): `zora-letter`,
   `jabu-fish` and `jabu-jabu` (and every preset built on it) apply the diving game's win:
   `Item_Give(ITEM_SCALE_SILVER)` (`UPG_SCALE` 1: `Item_Give` adds `item - ITEM_SCALE_SILVER + 1`)
   and `EVENTCHKINF_OBTAINED_SILVER_SCALE` (0x38), which `EnDivingGame_EndMinigameAfterScaleGiven`
   sets with it. `zora-diving`, `zora-bottle`, `boomerang` and `fairy-magic` stay without it.
2. **Ruto whole**, by a helper: every type (Barinade's room, the fountain, Jabu-Jabu's holes room,
   basement and sapphire room, beside King Zora, beside the door switch, the debug type), her three
   overlay cutscenes (already in the pack), carried and thrown through Player's generic lift,
   her bounces, her water (bobbing, sinking, fading), the switches she weighs down, Big Octo's
   platform through `BgBdanObjects_GetProperty/_SetProperty`. `Door_Warp1` gains the
   `rutoWarpState` field she reads and writes; its Ruto states (`DoorWarp1_RutoWarpIdle`,
   `_RutoWarpOut`) are milestone 6's with Barinade.
3. **Player's Ruto checks:** a door opens with Ruto carried (`Player_ActionHandler_1`'s
   `heldActor->id == ACTOR_EN_RU1`; the do-action already allowed it), and she cries on Link's
   falls (`func_80843E14`: `Actor_PlaySfx(heldActor, NA_SE_VO_RT_FALL)`, a new
   `PlayerSfx::AtActor`).
4. **The tentacles, barriers, crates and timers whole**, by a second helper. `En_Bx` zeroes and
   restores Link's invincibility timer to hurt him: a new `PlayerIface::set_invincibility_timer`.
   `Obj_Roomtimer` changes its category to a prop on its first update, and when it's the room's
   last enemy that change clears the room (`Actor_RemoveFromCategory`'s check, which the earlier
   ports skipped: `obj_roomtimer::actor_change_category`).
5. **A cleared room on a debug start brings its timer's flag.** A room timer is an enemy until
   its first update, so a room marked cleared (`Flags_SetClear` before the room loads) never spawns
   it to set its flag; a save that beat the room has the flag (switch flags under 0x38 are kept).
   `Route::JabuWeb` sets room 11's 0x01 (the golden torch's) with its clear.
6. **The way, settled** (the search over the floors, line tests, and the runs):
   - **Room 2:** Ruto's meeting starts when Link is within 265 of her along z. Six holes drop to
     room 3 (x -80 to 0 at z -3040 to -3120, 180 to 220 at -3200 to -3240, 0 to 80 at -3280 to
     -3360, two south-west); the floor's conveyors carry Link to them. The cow's niche is in the
     north wall 156 over the floor, behind three boulders (`Obj_Bombiwa` 0x06, 0x11, 0x1A), which
     break only to an explosion within 80 (`Actor_FindNearbyExplosive`). Standing still, A puts a
     carried bomb down (`Player_CanThrowCarriedActor`); so the bomb is held until its fuse is down
     to 30, then thrown at a run 130 south of the niche (at Link's speed plus 8, 12 up): it bursts
     against the niche's wall 50 under the boulders. The cow's target is shot from 330 back (nearer,
     the seed meets the niche's lip). What looks like a 60 rise in front of door 4 is an arch.
   - **Room 11 to room 7's web:** a lit stick burns 210 frames (`unk_860`). From the torch, through
     door 7 and across room 7 to the web takes about 180 at a run: it lasts. Room 7's floor
     conveyors (speed 2, towards its middle) beat a slow walk: the web is reached at a run, and the
     stick's tip crosses its plane (`BgYdanSp_WallWebIdle`: z under 1 in the web's frame).
   - **Room 5:** door 9's floor (-320) drops to the pool (its floor -440, the water 20 deep). The
     crystal (`Obj_Switch` 0x1403, its sphere 20 across at y -30) is shot from the pool with the
     slingshot, aimed 8 over it for the seed's fall. Link didn't take the step the search found back
     up to the south floor (62 at its lowest; where he met it, 67 to 71, pushing into it for 900
     frames; why wasn't looked into); the pool's south rim is a ramp from x 1700 (-391) up to x
     1460 (-320), which the run takes. Door 0 opens with the crystal's flag.
   - **Room 15** is nothing but exit 2's floor (x 1220 to 1500, z -2119 to -2399): through door 0,
     Link falls into Barinade's room.
7. **The runs chain by presets:** `jabu-boomerang` → `Route::JabuRuto` → `jabu-room-7` →
   `Route::JabuWeb` → `jabu-web`; `jabu-crystal` (a debug start: the way's two tentacles, gray 0x0A
   and red 0x0B, and room 7's boulders 0x1C, as the hand test's fights would leave them) →
   `Route::JabuCrystal` → `jabu-barinade`, milestone 6's start. Each end is checked against its
   preset, less the debug start's room clears.

## Consequences

- The story's saves can dive for Ruto's letter. Two goldens moved and were proven by run-time
  switches: `jabu_elevator` by the scale (its dive), `king_zora` by Ruto (her type 5 is placed
  beside the king and kills herself at init without `EVENTCHKINF_37`).
- In Master Quest Ruto's `INFTABLE_RUTO_PLACED_ON_SWITCH` is never set: she sets it only on a
  `Bg_Bdan_Switch` with flag 0x38, and MQ's room 3 switch is 0x3C (she still holds it down). Room
  7's Ruto (0x0206, which wants that flag) never appears; room 7 doesn't load her object anyway.
- Barinade's room is reached; `Boss_Va` (and the Ruto it spawns, with her warp) is milestone 6's.
- `Task::ThrowAtRun` steers at its last waypoint, not the release point: its waypoints must run
  past the release.
