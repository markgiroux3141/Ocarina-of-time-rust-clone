# 0011: The collision check: colliders stay in their actors, the context holds references

- **Status:** accepted, built in GAME-02 milestone 1
- **Date:** 2026-09-28

## Context

Phase 3's actors need `z_collision_check.c`:
- the sword cuts bushes and signs (AT against AC);
- rocks bounce the sword off (`AC_HARD`);
- Link bumps into NPCs and props (OC).

In the game an actor embeds its colliders in its own struct and registers pointers to them each frame (`CollisionCheck_SetAT` / `SetAC` / `SetOC`). At the top of the next `Play_Update` the checks run over those pointers and write into both sides of every hit: flags, `atHit` / `acHit`, the hit elements, and `colChkInfo.displacement` for OC pushes.

ADR 0007 put actors in a generational arena, each taken out of its slot while it updates. A context of raw pointers into the actors is exactly what that design rules out.

## Decision

- **An actor keeps its colliders.**
  - The context (`oot_game::collision_check::CollisionCheckContext`) holds `ColliderRef { actor handle, id }` in its AT, AC and OC lists.
  - `ActorImpl::collider_mut(id) -> Option<ColliderMut>` lends a collider out by the id the actor registered it with.
- **The checks take the colliders out, run, and put them back.**
  - `CollisionCheckContext::check(&mut ActorContext)` takes every registered collider out of its actor and runs `CollisionCheck_AT`, `_OC` and `_Damage` over the owned set, in the C's list order and with its pair functions. Then it returns each collider to its actor.
  - What the check sees is whatever the actor left in the collider at registration or after, as with the pointers.
- **Cross-collider pointers become handles, and the data read through them is copied.**
  - `atHit`, `acHit` and `ocHit` are actor handles.
  - The hit element an actor reads later (`acHitInfo->toucher.dmgFlags`, the bumper) is copied into a `HitElem` when the hit is recorded. In the game it's read through the pointer, and it only changes if the other actor re-initialises its collider, which none of the ported actors do between a hit and its reading.
- **The frame is the game's.** `PlayState::tick_with` runs the checks after `Room_ProcessRoomRequest` and before `Actor_UpdateAll`, then clears the lists (`CollisionCheck_ClearContext`). After each actor's update, `CollisionCheck_ResetDamage` and the `targetPriority` reset run, as in `Actor_UpdateAll`.
- **The geometry is engine code.** The `sys_math3d.c` primitives the checks use (spheres, cylinders, triangles, segments) are `eng_collision::math3d`, which knows nothing about actors.

## Consequences

- Every shape pair of the C is ported: 16 AT-against-AC pairs, the quad's nearest-AC rule with its deferred resets, OC pushes by mass (`MASS_IMMOVABLE`, `MASS_HEAVY`), and damage from damage tables.
- **An actor with colliders implements `collider_mut`** and registers each collider with an id it answers to. Player uses 0 (body cylinder), 1 and 2 (the sword's quads) and 3 (the shield).
- **Not ported:**
  - the hit effects (blood, sparks, hit marks) and sounds: `CollisionCheck_HitEffects` keeps only its flag bookkeeping;
  - the SAC list mode and OC lines (unused), and `CollisionCheck_LineOC`;
  - colliders without an actor.
