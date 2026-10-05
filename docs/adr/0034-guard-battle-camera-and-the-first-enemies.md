# 0034: The guard, the battle camera and the first enemies

- **Status:** accepted, built in GAME-05 milestone 3a (2026-10-05)
- **Date:** 2026-10-05
- **Builds on:** [ADR 0007](0007-actor-ownership.md) (actors own their state, the arena),
  [ADR 0013](0013-camera-modes-and-screen.md) (camera modes, the fallback to `Camera_Normal1`),
  [ADR 0029](0029-one-point-cutscenes.md) and
  [ADR 0033](0033-effects.md) (the effects).

## Context

Milestone 3a's other parts:
- Player's guard (R) with the shield (BACKLOG #4);
- `Camera_Battle1`;
- `En_Karebaba` and `En_Firefly`, and drops on death.

The exit: Link blocks a hit, and fights a Keese and a withered Deku Baba with the battle camera;
the kills show effects and drop items. Six things needed a decision:
- **The held shield's model.** `Player_SetModelsForHoldingShield` swaps the right hand to
  `PLAYER_MODELTYPE_RH_SHIELD` and the sheath to its shield-less type. The pack baked Link's
  hand, sheath and waist lists per loadout, and the loadout had no "holding the shield".
- **Deflecting.** The deflection is the projectile's: `En_Nutsball`, the Deku Scrubs' nut, turns
  round when its attack bounced off the Deku Shield (`AT_BOUNCED`). Player's side is the shield's
  collider (`AC_HARD`, and AT). The nut is 3b's.
- **`Item_Shield`**, the burnt Deku Shield, was a placeholder.
- **`Camera_Battle1` in rooms with the skybox disabled** (the Deku Tree) calls `func_80043F94`
  for NORMAL0's BATTLE data (`BATTLE1_FLAG_1`). The camera port used `Camera_BGCheckInfo`
  everywhere.
- **`Actor_UpdateAll` and `Actor_ChangeCategory`.** A withered Deku Baba changes its category in
  its own update (to `ACTORCAT_MISC` when it dies, back to `ACTORCAT_ENEMY` when it regrows).
  - The C's loop then follows `actor->next` into the other category's list: the rest of the
    first list waits a frame, and the other list's actors update twice.
  - The port iterated a snapshot of each list.
- **The exit's scripted run** has to play against enemies whose moves depend on `Rand`.

## Decision

- **The held shield is a loadout bit.** `Loadout::holding_shield`, from Player's
  `holding_shield`, switches the right hand to `RH_SHIELD` and the sheath from 18 to 16 and 19 to
  17, as `Player_SetModelsForHoldingShield` does. The importer bakes each loadout with it on and
  off (146 Link variants). Player draws with a render switch, and registers the shield's quad
  from the hand's matrix (`Player_UpdateShieldCollider`) or the sheath's (the child's Hylian
  Shield), keeping `shieldMf` for the actors that read it.
- **`En_Nutsball` is pulled forward from 3b, whole,** so the deflection is tested with the real
  projectile. It's small, and 3b's scrubs spawn it.
- **`Item_Shield` is ported whole** (both of its kinds), replacing the placeholder.
- **`func_80043F94` is ported and `Camera_Battle1` calls it** where the C does. The other ported
  modes that call it with the skybox disabled (`Camera_KeepOn1`, `Camera_Parallel1`) keep
  `Camera_BGCheckInfo`, as before. That's a known gap, to be closed with their next
  change. `CamFrame` gains the room's `skyboxDisabled`, Player's `meleeWeaponState` and the
  health.
- **`Actor_UpdateAll` follows the C's list walk:**
  - when an actor's update changes its category, the loop goes on from it in the new list;
  - the rest of the old list waits a frame, with the old category's freeze mask;
  - the other list's actors update again in their own turn if it comes later.
- **The scripted runs play the enemies, not the clock.**
  - `Route::Combat` kills the withered Deku Baba first (while it's up, Z would lock on to it
    rather than the Keese, and nowhere in the Keese's reach is it outside the 60° that
    `Attention_WeightedDistToPlayerSq` looks in).
  - It then blocks the Keese's dive with the shield and slashes it while it hovers
    (`EnFirefly_Stay`).
  - It waits for the Keese's dives, since when they come depends on `Rand`.
  - The tests of each enemy's states drive them directly where a run can't reach a state
    reliably.

## Consequences

- The Deku Scrubs (3b) need only their own actors: the nut and Player's side are done.
- Pack format 18 (`out/data15`), together with ADR 0033's bakes.
- Every golden where Link is locked on to an enemy or a dummy changed (the battle camera), and
  so did every one where a slash meets a wall (`func_80842DF4`'s recoil, now ported). The
  category walk changes only runs with a withered Deku Baba or a Deku Baba dying.
- Long scripted runs against `Rand`-driven enemies are costly to write and to keep. From 3b on,
  each milestone keeps one short run as its exit and golden, and checks what plays right by
  hand (a list in the milestone's doc), with the C-derived tests per actor doing the rest.
