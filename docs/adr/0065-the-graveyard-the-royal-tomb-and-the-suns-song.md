# 0065: The graveyard: Player's grab, the royal tomb, the ReDeads and the Sun's Song

- **Status:** accepted, built in GAME-07 milestone 6 (2026-10-13)
- **Date:** 2026-10-13
- **Builds on:** [ADR 0061](0061-the-ocarina.md) (the ocarina, `En_Okarina_Tag`, the cutscenes'
  ocarina texts), [ADR 0062](0062-the-aim-master-quest-and-the-game-editor.md) (each actor's
  Params and Flags, COVERAGE), [ADR 0064](0064-kakariko-lifting-and-the-village.md) (the helpers'
  worktrees, `Actor_CleanupContext`).

## Context

Milestone 6 closes Phase 8: child Link in Kakariko's graveyard at night, a dig with Dampé, Zelda's
Lullaby at the royal tomb, the tomb's Keese and ReDeads, and the Sun's Song taught at its wall.

The survey (milestone doc, "Milestone 6") found:
- The graveyard's intro (`gGraveyardIntroCs`) and the Sun's Song's cutscene chain were already
  playing: the lesson's terminator (`CS_DEST_GRAVEYARD_SUNS_SONG_PART_2`) enters the graveyard's
  layer 5 (`gGraveyardSunsSongPart2Cs`), whose terminator enters `ENTR_ROYAL_FAMILYS_TOMB_1`, where
  the entrance table plays part 3 (`gSunSongGraveSunSongTeachPart2Cs`, `EVENTCHKINF_5A`). The
  tomb's opening is started by `En_Okarina_Tag` type 6 (ported in milestone 3).
- Not ported: `En_Cs`, `En_Tk`, `En_It`, `Bg_Spot02_Objects`, `En_Rd`, `Bg_Breakwall`,
  `En_Weather_Tag`, `Obj_Makekinsuta`, `En_Po_Relay`; `Object_Kankyo`'s types 2 (the lightning)
  and 4 (the tomb's spark); `EffectSsBomb2` (the gravestone's blast, BACKLOG #40); three scene draw
  configs (BACKLOG #46).
- A ReDead holds Link through Player's grab (`play->grabPlayer`, `func_80852F38`,
  `Player_Action_8084F308`) and locks him on to itself (`Player_SetAutoLockOnActor`,
  `autoLockOnActor` in `Player_UpdateZTargeting`): neither was ported.
- Dampé is out only after 18:00 (`EnTk_Init` kills him at `dayTime <= CLOCK_TIME(18, 0)`), and he
  walks the scene's path; he doesn't follow Link. The tomb's first door is `SHUTTER_FRONT_CLEAR`:
  its Keese have to die. `En_Po_Relay` (Dampé's race) is in Dampé's grave, which a child can't
  open (`Bg_Spot02_Objects` type 1 seals it for him).

## Decision

1. **The scope** (the user's calls, 2026-10-13): one milestone, in steps; `Obj_Makekinsuta` ported
   now (its bug's write a public field, `unk_152`, for `En_Insect` to set); `En_Po_Relay` left for
   Phase 14, where the ROADMAP has Dampé's race (`scripts/coverage.py`'s `ACTORS` says so);
   `EffectSsBomb2` ported now.
2. **Player's grab and auto lock-on from another actor's update.** `play_grab_player(play)` is
   `play->grabPlayer` (`func_80852F38`: the held action `Player_Action_8084F308`, the struggle with
   `func_80832594`, the cry); `player_set_auto_lock_on_actor(play, actor)` is
   `Player_SetAutoLockOnActor`. Player is in the arena while the ReDead updates, so they reach him
   by downcast and apply his requests at once, as `play_damage_player` does. `autoLockOnActor` is a
   Player field: it keeps Z-targeting alive, isn't let go by its distance, takes the place of a
   missing focus actor, and is cleared at the end of every update, as the C does.
3. **The actors whole**, each with its Params and Flags paragraphs: `En_Cs`, `En_Tk`, `En_It`,
   `Bg_Spot02_Objects` (all five types), `En_Rd` (all seven types, the Gibdos too),
   `Bg_Breakwall` (all four walls, King Dodongo's too), `En_Weather_Tag` (all eight types),
   `Obj_Makekinsuta`; `Object_Kankyo`'s types 2 and 4 (3, the snow, and 5, Ganon's castle's beams,
   wait for their phases).
4. **`EffectSsBomb2` whole** (`oot_game::effect::bomb2`: its fade and layered draws), spawned by
   the gravestone, and now also by the two actors that logged it (`En_Gs`'s launched stone,
   `En_Ex_Ruppy`'s bomb rupees), neither of which a run reaches.
5. **`Flags_GetSwitch`, `_SetSwitch` and `_UnsetSwitch` wrap like the C's shift** (@bug (game)): a
   flag of 0x40 and up lands on temp flag `0x20 + ((flag - 0x20) & 0x1F)` (MIPS `sllv` takes the
   low 5 bits), so the tomb's ReDeads (0x7F) set temp flag 0x3F. Negative flags stay no-ops (the
   port's callers' "none").
6. **The weather tag wires BACKLOG #51:** `gInterruptSongOfStorms` is set and cleared as the C's
   tags do, and the Song of Storms' storm already read it.
7. **The draw configs** `SDC_GRAVE_EXIT_LIGHT_SHINING` (the ReDead grave, the windmill and Dampé's
   grave), `SDC_FAIRYS_FOUNTAIN` (the shield grave and the fairy fountains) and
   `SDC_ROYAL_FAMILYS_TOMB`, checked against the C interpreter.
8. **The exit's run** (`Route::Graveyard`, 18:30, `graveyard-lullaby`): the intro; Link waits on
   Dampé's path north of the dig spot at (-473, 60, 202), pays him (0x5019's yes) and picks up the
   prize; east between the grave rows and round the 60-high wall at x -42 (z -180 to 520); the
   lullaby within 50 of its spot (where the spot calls Link's ocarina to it); into the hole; the
   four Keese each approached until it dives, blocked and killed; up the door's notch (a 60 ledge,
   then 30 steps); through room 1 at a run but more than 150 across from every ReDead (idle ones
   wake only to Player's noise, `PLAYER_STATE2_3`, within 150); the wall's lesson and the song
   played back; the chain back to the wall; and the Sun's Song played in room 1, where every
   ReDead is stunned. New tasks: `PickNearest`; `Who::Dampe` and `Who::SongWall`. Presets
   `graveyard-lullaby` (milestone 5's end) and `graveyard-sun` (this run's end).
9. **The work split** as milestone 5's: Player's grab, the draw configs, the route, the scripts and
   the docs here; the actors by five helpers in their own worktrees, merged and checked here.

## Consequences

- Any actor can now hold Link (`play_grab_player`) or lock him on to itself; the Like Like and
  the Dead Hand will use the grab. The hookshot's checks in `func_80852F38` wait for the hookshot.
- A switch flag of 0x40 and up now sets or reads a temp flag where it did nothing: no golden
  changed.
- `Bg_Breakwall` stands and blocks the shield grave's fairy fountain until the bombs (`En_Bom`,
  BACKLOG #33).
- The kid's Spooky Mask trade and Dampé's and the kid's mask reactions wait for the masks (#39);
  the invisible ReDead waits for the Lens of Truth; rumble isn't ported.
- Phase 8 is done but for the Temple of Time's room 0 (BACKLOG #37: `Bg_Toki_Swd`, `Demo_6K`,
  `En_Xc`), which waits for the Door of Time.
