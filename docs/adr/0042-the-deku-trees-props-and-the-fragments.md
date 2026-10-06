# 0042: The Deku Tree's props and the fragments

- **Status:** accepted, built in GAME-05 milestone 4b (2026-10-06)
- **Date:** 2026-10-06
- **Builds on:** [ADR 0012](0012-actor-bakes.md) (bakes), [ADR 0033](0033-effects.md) (effects),
  [ADR 0038](0038-sliding-doors-and-room-travel.md) (the interact flags) and
  [ADR 0039](0039-quakes.md) (quakes).

## Context

Milestone 4b's props: `Bg_Ydan_Hasi` (room 5's floating block and water, room 10's rising
platforms; only the block was ported, built directly by the sandbox), `Bg_Ydan_Maruta` (room 5's
spiked log, room 2's ladder), `Obj_Kibako2` (the large crates, one shutting room 0's Gold
Skulltula in), `Obj_Lift` (room 2's collapsing platform), and `Effect_Ss_Kakera`, the fragments
the crates and the lift break into, which `En_Kusa`, `En_Ishi` and `En_Goroiwa` (ported) also
spawn. They were ported by two worktree agents in parallel with the Deku Stick.

Five things needed a decision:
- **Water boxes written at run time.** `Bg_Ydan_Hasi` writes `waterBoxes[1].ySurface` of the
  scene's collision header every frame; the engine's header was read-only.
- **`Effect_Ss_Kakera`'s tables** are one block of data in the overlay, and some of its index
  computations read past a table into the next.
- **Untriggerable breaks.** The crates break only on the hammer or an explosion
  (`func_80033684`), the ladder only on a seed; none is Link's yet.
- **The sandbox's platform** was a hand-built floating block.
- **The pack** needs the fragments' lists drawn with their colour.

## Decision

- **Each actor and effect is ported whole**, their triggers injected in the tests: a test-local
  `ACTORCAT_EXPLOSIVE` actor with params 1 in reach (the C's path through `func_80033684`), a
  hammer's `AC_HIT`, a seed's `DMG_SLINGSHOT`, room 5's water flag 0x3F (never set in MQ).
- **The scene's water boxes are written in place** on the play state's own collision header
  (`CollisionContext::set_water_box_surface`), which Player's water checks read; a scene reload
  restores them, as the C's reload does.
- **`BgYdanHasi::init_with(actor, play, header)`** lets the course (no objects or assets) run the
  real init; `PlayExt::spawn_platform` builds the block as `Actor_Spawn` would (params 0xFF00) on
  the course's `waterBoxes[1]`. The slide's argument is computed in double, as the C's `M_PI`
  makes it.
- **`Effect_Ss_Kakera`'s data is one contiguous table**, so an index past one table reads the next
  as the game does (`@bug (game)` where it matters); the one read that would call through a float
  as a function pointer logs and uses 1.0.
- **Each fragment list is a bake with a dynamic primitive colour** on segment 0x0E (seven lists:
  the crate's, the lift's, the bush's stalk and tip, the rocks', the boulder's). Pack format 21
  (`out/data18`).
- **`z_actor.c`'s `func_80033480`** (the dust puffs) and **`func_80033684`** (an explosive in reach),
  and `BgCheck_SphVsFirstPoly` (the fragments' floor check), are ported; `En_Kusa`,
  `En_Ishi` and `En_Goroiwa` spawn their real pieces, with the C's `Rand` order (several calls in
  one argument list assumed left to right, as before).

## Consequences

- Room 5's log spins and knocks Link down, its block floats on the water; room 10's floor switch
  raises the three platforms for 260 frames; room 2's lift shakes, falls and breaks; the crates
  are solid and drop a green rupee when broken; the ladder falls on a seed.
- A cut bush scatters its leaves, and their `Rand` comes before its drop: Kokiri Forest's runs
  change (the `playthrough` golden's first bush now drops).
- Hits aren't blocked by background collision (in the C or here): the sword still reaches room
  0's Gold Skulltula through its crate.
- Bombs and the hammer (later milestones) break the crates and rocks through the same paths the
  tests inject.
