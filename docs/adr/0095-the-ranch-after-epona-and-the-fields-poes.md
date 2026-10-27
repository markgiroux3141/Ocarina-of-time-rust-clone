# 0095: The ranch after Epona and Hyrule Field's Poes

- **Status:** accepted, built in GAME-12 milestone 4 (2026-10-26)
- **Date:** 2026-10-26
- **Builds on:** [ADR 0091](0091-riding-epona.md) (`En_Horse`, Player's riding),
  [ADR 0092](0092-ingo-the-rental-and-the-races.md) (`En_Horse_Game_Check`'s obstacle course),
  [ADR 0093](0093-epona-won-and-out.md) (`epona-won`).

(ADR 0094 is the level editor's Reference mode, written in the same tree at the same time.)

## Context

Phase 13's last milestone: the ranch once Epona is Link's (adult Malon's obstacle course, Malon
singing by night before Epona) and Hyrule Field's Poes. The course's referee (type 4) came whole
with milestone 2, and its fences with milestone 1. What was missing:
- **`En_Ma3`** (adult Malon by day after Epona: the course's offer and its results, the high
  score, the cow);
- **`En_Ma2`'s adult** (a placeholder since Phase 9: the stable by day, the ranch by night, Epona's
  Song played to her);
- **`En_Po_Field`** (the field's ten Poe places, one actor serving them all, the Big Poe and its
  soul in a bottle).

The message box printed every timer and high score as zeros, and Malon's texts show both.

## Decision

1. **`En_Ma3`, `En_Ma2`'s adult and `En_Po_Field` whole**, ported by two helpers in parallel. Adult
   Malon's one model (`object_ma2`) is baked once, in `en_ma2.rs`, and `en_ma3.rs` draws with it.
   `En_Po_Field`'s overlay statics follow ADR 0059 (dropped with the last instance).
2. **`Message_Decode`'s `MESSAGE_RACE_TIME`, `MESSAGE_MARATHON_TIME` and `MESSAGE_HIGHSCORE`**: the
   timers as mm"ss", the high scores from the save as counts or times by their kind (`HS_*`).
   `MESSAGE_POINTS` and `MESSAGE_FISH_INFO` still print 0.
3. **The runs** (short debug starts, no fights):
   - `malon-course` (`epona-won` by Malon): her thanks, Epona's Song, the course taken, its intro to
     its start. The laps are the hand test's (the user's call 6).
   - `malon-record` on the new debug preset **`malon-record`** (`epona-won` with the course just
     ridden in 45 seconds, at `ENTR_LON_LON_RANCH_7`): the record, the cow's unlock, the high score.
   - `malon-song` (`adult-awake` at 22:00): Epona's Song played to Malon, her talk.
   - `big-poe` (`epona-won` in the field): on Epona to the place by the ranch's ramp; the Big Poe
     appears and flees. The fight and the bottle are the hand test's.
4. **Pack format 36** for the two actors' bakes.

## Consequences

- Phase 13's four milestones are built.
- The Big Poe's soul (`ITEM_BOTTLE_BIG_POE`) waits for Phase 20's Poe collector.
- Child Malon (`En_Ma1`) still takes `MaskReaction_GetTextId` as 0 (BACKLOG #112). The Poe's
  light and its lantern's pieces share `En_Poh`'s gaps (BACKLOG #80, #81).
