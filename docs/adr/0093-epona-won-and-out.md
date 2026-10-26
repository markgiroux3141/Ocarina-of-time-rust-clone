# 0093: Epona won and out

- **Status:** accepted, built in GAME-12 milestone 3 (2026-10-26)
- **Date:** 2026-10-26
- **Builds on:** [ADR 0091](0091-riding-epona.md) (`En_Horse`, `z_horse.c`),
  [ADR 0092](0092-ingo-the-rental-and-the-races.md) (Ingo's gate scene, `Bg_Ingate`, the race's
  presets).

## Context

Phase 13's exit: Epona won. After the second race Ingo shuts the ranch, and Link leaps out on Epona
into Hyrule Field, where a cutscene plays and Epona becomes his. Milestones 1 and 2 had ported the
pieces: the trapped Epona, the cutscene horse and the four leaps' spawns (`z_horse.c`), `En_Horse`'s
cutscene actions, the gate scene (`En_In`) and the gates (`Bg_Ingate`). The entrance cutscene table
(GAME-03) holds the leaps' four rows, with `EVENTCHKINF_EPONA_OBTAINED`.

## Decision

1. **The run from the second win**, `Route::EponaWon` on the preset **`ingo-second-win`** at
   `ENTR_LON_LON_RANCH_7` (where the referee sends Link): the gate scene, a run-up, a spurred
   gallop over the shut gate past Ingo (`EnIn_GuardingGate`'s `ENTR_HYRULE_FIELD_15`), the leap's
   cutscene. Its end is **`epona-won`**, Phase 14's start. The fence leaps (`_11`, `_12`, `_13`)
   are a test from their entrances and the hand test's.
2. **`Task::RideUntil`**: ridden through points without stopping, spurring from any gait when
   lined up (`EnHorse_CheckBoost` takes a walk, a trot or a gallop), until a condition.
3. **`Play_LoadToLastEntrance`'s fence-jump case** (`OOT_VERSION >= PAL_1_1`, the debug ROM's):
   after a leap, a fall respawns at `ENTR_HYRULE_FIELD_6`. Ganon's collapsing tower's case waits
   for Phase 19.

## Consequences

- Phase 13's exit is met; Phase 14 starts from `epona-won`.
- Milestone 4 (the ranch after Epona, the field's Poes) is the phase's last.
