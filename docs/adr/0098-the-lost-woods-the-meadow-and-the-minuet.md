# 0098: The Lost Woods, the meadow and the Minuet

- **Status:** accepted, built in GAME-13 milestone 2 (2026-10-27)
- **Date:** 2026-10-27
- **Builds on:** [ADR 0096](0096-the-hookshot-and-dampes-race.md) (the hookshot, the `hookshot`
  preset), [ADR 0089](0089-the-door-of-time-to-the-chamber-and-out.md) (`En_Xc`'s shared song
  actions), [ADR 0090](0090-the-prelude-and-time-travel-both-ways.md) (the warp songs).

## Context

Phase 14's second milestone (GAME-13): adult Link's way from the hookshot to the Forest Temple's
door. The Lost Woods' adult Mido and the warp songs were ported; the meadow's Moblins (`En_Mb`),
Sheik's Minuet (`En_Xc` type 6, a placeholder) and Kokiri Forest's ring of rupees (`Obj_Mure3`)
weren't. The survey left open whether the meadow's tree takes the hookshot, and which type the
Spirit Temple's Moblin is.

## Decision

1. **The survey's points, checked in the pack:** the tree at the meadow's top takes the hookshot all
   over (73 of the meadow's 768 polys do: its trunk, its branch over the temple's ledge, a stump and
   the maze's four target faces); the ledge (a floor at 680, 200 over the top) is reached by the
   branch, as in the original: the flight ends with Link dropping onto it. The Spirit Temple's
   Moblin (`jyasinzou` room 13) is 0x0000, the club type, as the meadow's.
2. **`En_Mb` whole** (a helper): the three types (the spear guard, placed nowhere; the club Moblin;
   the spear patrols on the scene's paths), `EnMb_IsPlayerInCorridor`'s sight, the charge carrying
   Link, the club's strike, quake and shock wave (`En_Part` -1), the stun, the ice, the deaths. Its
   draw-time state (the attack quads, the front triangles, the club's `effSpawnPos`) runs in
   `draw_update`, as the C's `PostLimbDraw` does. Two skeleton bakes.
3. **`En_Xc` type 6** (the main session's): `func_80B3CA38`, `EnXc_MinuetCS` (z -2225, the script,
   `EVENTCHKINF_50`, `Item_Give(ITEM_SONG_MINUET)`), `EnXc_BoleroCS`'s pass for the other types, and
   actions 0 to 5 (the fall from the sky on her root's motion, `AnimTask_ActorMovement` as `En_Ru1`'s,
   the walk, the halt); actions 6 to 18 were ported with the Prelude. The Desert Colossus's sounds in
   actions 0 to 2 log there (Phase 18's).
4. **`Obj_Mure3` whole**; `Item_DropCollectible2` moves to `en_item00.rs` beside
   `Item_DropCollectible`.
5. **The runs:** `lost-woods-adult` (`hookshot`: Saria's Song to Mido, on into the meadow),
   `minuet` (a debug start on the ramp: Sheik's lesson), `minuet-warp` (`minuet`: Hyrule Field to the
   meadow's pad) and `forest-temple` (a debug start under the ledge: the hookshot into the branch,
   the ledge, the temple). The preset **`minuet`** (`hookshot` with `EVENTCHKINF_0A`, `_50` and the
   song). The maze past the Moblins is the hand test's (the user's call 5).
6. **Pack format 38** for the Moblins' bakes (`out/data30` imported).

## Consequences

- Adult Link reaches the Forest Temple by the story's way; milestone 3 (the temple's front and the
  bow) starts from `minuet` at `ENTR_FOREST_TEMPLE_0`.
- The Spirit Temple's club Moblin is in for Phase 18.
- `Obj_Mure3`'s Kokiri Forest ring hangs 240 over the bean's spot: its rupees are the hookshot's or
  the adult bean lift's (BACKLOG #29, later).
