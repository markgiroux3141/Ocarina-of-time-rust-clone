# 0086: The rest of Jabu-Jabu's Belly: Big Octo, the Bari, the Tailpasarans and Ruto's way to the Sapphire

- **Status:** accepted, built in GAME-10 milestone 8 (2026-10-21), hand-tested 2026-10-22
- **Date:** 2026-10-21
- **Builds on:** [ADR 0083](0083-jabu-jabus-belly-to-barinade.md) (Ruto whole, the way through
  Jabu-Jabu), [ADR 0082](0082-jabu-jabus-belly-to-the-boomerang.md) (the platforms and spouts, the
  search over the floors), [ADR 0066](0066-the-actor-schema.md) (the schemas),
  [ADR 0079](0079-the-level-texts.md) (the texts).

## Context

Phase 11's eighth milestone (`docs/GAME-10-zoras-waters-and-jabu-jabu.md`): the breadth of
Jabu-Jabu's Belly (Master Quest) off the way to Barinade. Three overlays were missing: `En_Bigokuta`
(Big Octo, 921 lines, spawned by `Bg_Bdan_Objects`' octo platform, which had a placeholder),
`En_Vali` (the Bari, room 6, 768) and `En_Tp` (the Tailpasarans, room 12, 729); and the platform's
three one-point cutscenes (3070, 3080, 3090) were logged as unknown. Every other actor rooms 3, 4,
6, 10, 12 and 13 place was already ported (`ootx scene-info --scene bdan`, every layer). Ruto's
room-6 scripts had only been driven in her tests, never reached with her carried, and BACKLOG #104
(her 71 jump as her walk to the Sapphire starts) was left to check here.

## Decision

1. **`En_Bigokuta` whole**, by a helper: his params-0 arrival (waiting for the platform's 1,
   the jump to its rim, params 2 for the platform's battle), params 3 (met before: fighting at
   once), the circling, the turn about, the stun (a Deku nut 20 frames, else 80), the spin and
   dizzy frames, the bomb's bounce, the hurt, the sink and rise, his death (`Flags_SetClear`,
   `CAM_SET_DUNGEON0`, the drop table); his colliders (the hard front, the back's sphere) and damage
   table (only his back is hurt); the mini-boss music; `CAM_SET_BIG_OCTO` with `CAM_STATE_CHECK_BG`
   off. Limbs 10 and 15 set the environment colour in the draw, so his skeleton is three opaque bakes
   split there, and one translucent bake for his death's fade. Not ported: his `Rumble_Request`s.
2. **`En_Vali` and `En_Tp` whole**, by a second helper. The Bari: waiting unseen above until Link
   is within 150, the drop, the spin with its tentacles' quads swept from their limbs (in
   `draw_update`), back up when he's over 250 away; its 20-frame shock with the lightning; the
   sword sets off the shock (an electric backlash) then damages it, a nut or the boomerang stuns it,
   fire burns and ice freezes it, a seed spins it fast; dead, three Biri (`En_Bili`). Its core,
   nucleus, hood and skeleton are separate bakes (the core with En_Bili's two combiners). The
   Tailpasaran: the head spawns its seven segments as a chain (each the last one's child); it
   rises, hovers, attacks and dives into the floor to come back at its place; any segment's last hit
   kills the whole chain from the head, segment by segment, each leaving a fading remnant; a nut
   stuns the whole chain. A segment finds its head up the parent chain (the C's `headInstance`: the
   head has no handle during its own init).
3. **One-points 3070, 3080 and 3090** in `OnePointCutscene_SetInfo`: their keyframe tables
   (`D_801215E4`, `D_80121774`, `D_80121814`, already in the pack), 3070's `QUAKE_TYPE_3` on the
   sub camera, 3090's Player hold. The platform's calls were already the C's.
4. **The way through room 3 with Ruto, settled by the runs** (the floors probed, then Link driven):
   Ruto sits on room 3's east floor (-1113), which the pool (its floor -1209, 3 deep) cuts off from
   the rest; the west ledge (-1015, door 21's) and the south floor (-1113, the blue switch's) have
   slopes for sides that Link slides down, not up. The way is the yellow switch 0x3A in the pool,
   over the spout (`En_Siofuki` 0x0EBF): carrying Ruto onto it, the spout lifts Link to about -1000
   for 300 frames, and a run west off its top lands on the ledge (Wikibooks' Master Quest
   walkthrough: "step on the yellow switch ... jump over to the highest platform you can reach").
   From the ledge, its south slope down to the blue switch (`Route::JabuSwitch`), or west past the
   green barrier (gone with 0x0C) through door 21 into room 6 (`Route::JabuOcto`).
5. **Room 6 as the C runs it, with Ruto carried in:** `gRutoFoundSapphireCs` (0x404F) as she's
   carried in with the Sapphire there; thrown onto the platform, `gRutoObtainingSapphireCs` (0x4050
   to 0x4052, `INFTABLE_RUTO_HAS_SAPPHIRE`, her switch flag 0x02); the platform waits for Link within
   250 (3070), rises; at its top 0x4048, Big Octo spawned on it (`INFTABLE_RUTO_ABDUCTED`), the
   descent with him (3080), Link put at (-1130, -1025, -3500), and Big Octo's params 0 → 1 → 2: the
   battle.
6. **BACKLOG #104 closed:** the C does the same. `Animation_Change` loads the walk's frame 0 into the
   joint table and her `prevTransl` is still the spawn's zero, so her first `AnimTask_ActorMovement`
   moves her by the root's whole offset; the cue's spot is the walk's end (at the Sapphire) and she's
   hidden until that frame, so nothing is seen to jump.
7. **Two runs, no fights** (`Route::JabuSwitch`, `Route::JabuOcto`), both from room 3 by Ruto on
   `jabu-room-7` with room 3 marked cleared; `JabuOcto`'s debug start also sets the green tentacle's
   flag 0x0C. `Task::LiftWho` lifts an actor by `Who` (`Who::Ruto`); `Task::PutDown` presses A again
   if a switch's camera took the first. Presets: `jabu-sapphire` (a debug start: the Sapphire
   Ruto's, not yet taken; in room 6 she waits on the platform) and `jabu-big-octo` (`Route::JabuOcto`'s
   end: Big Octo's fight's start; re-entered, the platform spawns him at once).

## Consequences

- Every actor Jabu-Jabu's Belly (Master Quest) places or spawns is ported; Phase 11's left are the
  frogs' and scarecrows' (milestone 9).
- Big Octo's fight, the Bari's and the Tailpasarans' are the hand test's (`game-jabu.bat`'s
  `bigocto`, `bari`, `tailpasaran`), as are Ruto's way up the spout (`octo`) and the platform with
  her on it (`platform`).
- The game's bugs kept (`@bug (game)`): `func_809BD6B8`'s `ABS(shape.rot.y - yawTowardsPlayer)` taken
  as an int, unwrapped; on Big Octo's death frame `CAM_SET_DUNGEON0` and `CAM_STATE_CHECK_BG` are set,
  then the same update's `CAM_SET_BIG_OCTO` is refused (both priority 5) and `CAM_STATE_CHECK_BG`
  unset: the camera ends on `CAM_SET_DUNGEON0` without checking the bg. The Bari's tentacle length
  steps in each tentacle root's limb override, twice a drawn frame, the right one a step ahead. The
  Tailpasaran's stun plays its freeze sound at the part hit once for each parent up the chain.

## After the hand test (2026-10-22)

- **The renderer's `LOD_FRACTION`** (the blue warp looked cut in the middle): the RDP computes it
  whenever the combiner reads it; with sharpen and detail off and `gSPTexture`'s level 0 every pixel
  is "distant" and it's 0xFF. Materials carry that static value (`Material::lod_frac`, pack format
  31); the per-pixel case is BACKLOG #109. `gWarpPortalDL`'s TEXEL1 is its texture, TEXEL0 the same
  shifted half its height (its second load wraps round TMEM), so the warp now shows TEXEL1.
- Room 4's floating Flobbery voiding Link out is the C's own behaviour, kept (BACKLOG #107).
- **The stutter with a C button while locked on** (BACKLOG #108): Player's item raise and its
  first-person action now call the whole `Player_UpdateHostileLockOn`, as the C does; the clearing
  half alone dropped the hostile lock-on and set the camera flipping into the first-person aim and back.
  Diagnostic log lines kept: `[target]`, `[camera]`, `[frame]` at info, `[sfx] start` at debug.
