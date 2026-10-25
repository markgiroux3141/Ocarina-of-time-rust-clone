# 0092: Ingo, the rental and the races

- **Status:** accepted, built in GAME-12 milestone 2 (2026-10-25)
- **Date:** 2026-10-25
- **Builds on:** [ADR 0091](0091-riding-epona.md) (riding, `En_Horse` with its race actions),
  [ADR 0070](0070-lon-lon-ranch-the-horses-ingo-and-the-guays.md) (the child's Ingo, the adult's
  placeholders).

## Context

Phase 13's second milestone: Ingo's rental and the two races. `En_In` was ported for the child in
Phase 9, with the adult's texts but none of his actions: where the first update would start an
adult's role, the init spawned a placeholder. `En_Horse`'s race actions came with milestone 1.
`Bg_Ingate` (the ranch's gates) and `En_Horse_Game_Check` (the race's referee) weren't ported.

## Decision

1. **`En_In`'s adult whole**, and the init's placeholder gone: the rental, the race's rider, the
   outcomes, the second win's gate scene and the guarding. His obedient and sleeping roles come
   with it (idle animations, nothing more to port). The adult's 22 lines join his schema's texts.
2. **`Bg_Ingate` and `En_Horse_Game_Check` whole**, by a helper in parallel: all four referee
   types (Malon's course included, for milestone 4); Gerudo archery's start sets the horse's
   `hbaStarted` and goes no further (Phase 18).
3. **The runs stop short of the race** (the user's call 3): `Route::IngoRental` (the rental to
   Epona mounted), `Route::IngoRace` (the race taken from horseback, the intro, to the start), and
   the outcomes from the presets `ingo-race-lost` and `ingo-first-win` at `ENTR_LON_LON_RANCH_8` and
   `_7`, where the referee sends Link (spawn 0 would reset the race's word: `sIsObedient`). The
   rental's minute is a test, not a run (the user's call 5). `ingo-rental` holds 60 Rupees as a
   debug start's (the user's call 4).
4. **`Task::RideTalk`**: on the horse, to a point at a walk (she halts at once), halted, turned in
   place to the speaker, A only on the talk offer (A standing on the horse is "Down"), then as
   `Converse`.
5. Shared constants: `oot_game::horse` gains the rental's and outcomes' entrances, the circle
   transitions' types (`trans_type_circle`, `TCA_*`, `TCC_*`, `TCS_*`) and the race's states;
   `oot_game::audio` the horse's music (`NA_BGM_HORSE`, `_HORSE_GOAL`, `_INGO`).

## Consequences

- Milestone 3 starts from the second win: the gate scene is ported and the gates swing; the jump
  out (`En_Horse`'s cutscene actions, `Horse_SetupInCutscene`'s trapped Epona) is what it adds.
- The race itself is the hand test's.
- Two game bugs in Malon's course are kept (`@bug (game)`): her race reads Link's horse without a
  check, and the first fence's foul reads `fenceCheck[-1]`.
