# 0072: Bombs and Bomb Flowers

- **Status:** accepted, built in GAME-09 milestone 1 (2026-10-15)
- **Date:** 2026-10-15
- **Builds on:** [ADR 0064](0064-kakariko-lifting-and-the-village.md) (Player's lift, carry, throw
  and put-down), [ADR 0033](0033-effects.md) (the effects), [ADR 0065](0065-the-graveyard-the-royal-tomb-and-the-suns-song.md)
  (`EffectSsBomb2`), [ADR 0066](0066-the-actor-schema.md) (the schemas).

## Context

Phase 10's first milestone (`docs/GAME-09-death-mountain-and-dodongos-cavern.md`): the bombs every
later part of the phase stands on. The trail's boulder and Goron City's walls, Dodongo's Cavern's
entrance wall, its skull's eyes and most of its enemies, and King Dodongo all take a bomb or a Bomb
Flower's blast. BACKLOG #33 had waited for it since GAME-06: the Peahat's death and the courtyard
guard's throw spawned placeholders, and the walls, boulders, crates, gossip stones and the Wolfos
that react to blasts were only ever tested with an injected one.

`En_Bom` (396 lines) and `En_Bombf` (523) are small, but they reach into Player: a bomb is spawned
in Link's hand from his item (`Player_InitExplosiveIA`, in the middle of his update), a Bomb
Flower hands his carry on to the bomb it spawns (`func_8002F5C4`), and a bomb that goes off in his
hand lets go of him itself. The fuse's sparks are `Effect_Ss_G_Spk` (153), not ported.

## Decision

1. **`En_Bom` and `En_Bombf` whole**, with their schemas (`En_Bombf`'s wall and ceiling flowers'
   `shape.rot.x` a rotation field), their bakes (the bomb's cap and body; the flower's leaves, and
   its bomb with the spark on segment 8's billboarded matrix, `EnBombf_NewMtxDList`, a list of one
   `gSPMatrix` on bone 0) and `Effect_Ss_G_Spk` whole (its unused update mode and spawners too).
2. **The explosion sphere where the bomb last drew:** `Collider_UpdateSpheres` runs in the C's
   draw while it's a bomb, so the port does it in `draw_update`; a bomb that blows before it has
   ever drawn keeps its sphere at the origin, as the C's would (no game spawner does that: they
   spawn in an update, and `Actor_DrawAll` draws the bomb the same frame).
3. **Player's explosive in hand through a predicted handle.** `Player_InitExplosiveIA` spawns the
   bomb as Player's child in the middle of his update, and the rest of that update reads it
   (`Player_UpperAction_CarryActor`'s `heldActor` check). Player can't spawn while he updates (he
   holds the play state read-only), so he takes the handle the next spawn will get
   (`ActorContext::next_handle`, read as his update starts) as `heldActor` and
   `interactRangeActor` at once, sets the carry and the rest of the C's writes, and asks for the
   spawn (`PlayRequest::SpawnHeldExplosive`). His update's end spawns it (`Actor_SpawnAsChild`),
   spends its ammo, and puts the real handle in place of the predicted one (or undoes the hold
   if the spawn failed, the C's NULL). The bomb still updates the same frame, in its category's
   turn after Player's, as in the C.
4. **The actors' writes into Player** as `PlayerIface` methods: `held_actor`,
   `hand_over_held_actor` (`func_8002F5C4`, now `oot_game::actor_ctx::func_8002f5c4`) and
   `clear_carried` (an explosive gone in his hand, a flower letting go). `holds_actor` reads
   `heldActor` (it had been `false`), and `Actor_OfferGetItem` checks
   `Player_GetExplosiveHeld`.
5. **The callers:** the Peahat's death spawns a bomb with no fuse (BACKLOG #33 closed), the
   courtyard guard's thrown bomb is a bomb; the walls, boulders, crates, gossip stones, the
   Wolfos and `En_Kanban` meet real blasts.
6. **The run (`Route::Bombs`) at Dodongo's Cavern's entrance**, from a debug start
   (`cavern-bombs`: `ranch-epona` with the Goron's Bracelet and the 20 bomb bag): the cavern's
   lobby is the first place both kinds of explosive are, and its entrance wall's opening is the
   phase's own. The story's way there (the bracelet in Goron City, the trail's boulder) is
   milestone 2's; the bag's chest is past the pit, milestone 3's.

## Consequences

- The phase's later milestones have their explosives: the boulder's bomb flower, the cavern's
  walls, eyes and pillars, the Dodongos, Beamos and Armos, King Dodongo's meals.
- `Bombchus` (`En_Bom_Chu`) aren't ported: Player's Bombchu item would spawn a placeholder
  (BACKLOG #70).
- Two Player fixes came with it (each a difference from the C): `holds_actor` always false (an
  item offered while Link carries something was refused), and an offer straight in front of Link
  (a yaw difference of 0x8000, whose `ABS` is 32768 in the C's `s32`) refused by the port's `i16`.
- The predicted handle is only for the frame between the spawn's call and its run; nothing else
  in the port spawns from inside Player's update this way.
