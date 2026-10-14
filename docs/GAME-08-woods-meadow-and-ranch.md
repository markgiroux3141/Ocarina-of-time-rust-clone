# GAME-08: The Lost Woods, the Sacred Forest Meadow and Lon Lon Ranch

**Goal:** Phase 9 of the [roadmap](ROADMAP.md): child Link with Zelda's letter in the Lost Woods,
through the Sacred Forest Meadow's maze to Saria and her song, and at Lon Lon Ranch for Malon,
Epona's Song and the child's Epona, Ingo, and Talon's house. Exit: Saria's Song learned.

**Status:** decided (2026-10-14). The user chose:
- the three milestones below, the exit first;
- every game and shop the phase's scenes hold: the Skull Kids' memory game, the Lost Woods' target
  scrub and its seed bag, Talon's milk and Super Cucco game (with the HUD timer);
- GAME-07's scope rule: every actor whole, except those a child's init kills (their init's checks,
  then a placeholder);
- the coverage table's corrections below.

| # | Milestone | Status |
|---|---|---|
| 1 | The meadow and Saria's Song: `En_Sa` (with Kokiri Forest's greeting), `En_Wf`, `Bg_Spot05_Soko`, the woods' maze walked | done (2026-10-14) |
| 2 | The Lost Woods' Skull Kids and scrubs: `En_Skj`, the memory game, the target scrub | planned |
| 3 | Lon Lon Ranch: Epona's Song's end, the foal and the horses, Ingo, the Guays, Talon's house and the cucco game | planned |

Before the phase, part A of the same piece of work: the actor schema ([ADR 0066](adr/0066-the-actor-schema.md),
BACKLOG #52), built and checked for all 131 ported actors (committed as 05f4f0e).

## The survey

Checked against the decomp (52a510f) and the pack, not against earlier notes. Each claim below was
read in the C; line numbers are the decomp's.

### The scenes and their layers

`ootx scene-info` now prints each layer's cutscene, which settled the cutscene layers:

| Scene | Layer | Cutscene | Phase |
|---|---|---|---|
| `spot10` (the Lost Woods) | 0, 1 | | 9 (child) |
| | 2, 3 | | 14 (adult) |
| | 4 | `gLostWoodsFairyOcarinaCs` (Saria's farewell on the bridge) | 7 (done, GAME-06) |
| `spot05` (the meadow) | 0, 1 | | 9 |
| | 2, 3 | | 14 |
| | 4 | `gMeadowMinuetCs` | 14 |
| | 5 | `gMeadowSariasSongCs` | 9 |
| `spot20` (Lon Lon Ranch) | 0, 1 | | 9 |
| | 2, 3 | | 13 |
| | 4 | `gLonLonRanchRaceIntroCs` (Ingo's race) | 13 |
| | 5 | `gLonLonRanchEponasSongCs` (Malon's lesson) | 9 |
| | 6 to 11 | `gLonLonRanchCreditsPart1Cs` to `Part6Cs` | 19 |
| `malon_stable`, `souko` | 0 to 3 | | 9 (child), 13 (adult) |

**Corrections to `scripts/coverage.py`'s `SCENES` (proposed):**
- `spot20`'s layer 6 is the credits' first part (its objects are Talon's, Ingo's, the Zoras', the
  Gerudos'), not the child's Epona: `{4: 13, 5: 9, 6: 9}` becomes `{4: 13, 5: 9}` (6 and up take
  the cutscene default, 19).
- `spot04` (Kokiri Forest): layers 10 and 11 are the credits' (`gKokiriForestCreditsDekuSproutCs`,
  `gKokiriForestCreditsCameraPanCs`), 12 and 13 the Deku Sprout's after the Forest Temple
  (`gKokiriForestDekuSproutPart1Cs`, `Part3Cs`); 8 and 9 hold unused cutscenes. Proposed:
  `{10: 19, 11: 19, 12: 14, 13: 14}`.
- `Obj_Dekujr` (the Deku Sprout) leaves Phase 3's list for Phase 14 (`ACTORS`): `ObjDekujr_Init`
  kills it for a child outside a cutscene layer and without the Forest Medallion
  (z_obj_dekujr.c, init). A child never meets it.

### What a child meets, and what's ported

**The Lost Woods** (child layers 0 and 1; the same placements):
- **Ported already:** the business scrubs (`En_Shopnuts`), a gossip stone, the bean soil and its
  bugs' spot (`Obj_Bean`, `Obj_Makekinsuta`), two fairy spots (`Shot_Sun` 0x41), the rocks and
  bushes, the owl (`En_Owl`), the grottos' holes (`Door_Ana`; the grottos are Phase 20), the wonder
  items, and in every room `En_River_Sound` 0x0C: Saria's Song heard louder towards the right way
  (`func_800F4E30` by distance, after Zelda's Lullaby and until Saria's Song is learned).
- **The maze is data, already working:** the rooms are joined by `En_Holl` planes, and a wrong
  turn is a collision exit (exit 1, ten polygons) back to Kokiri Forest (`ENTR_KOKIRI_FOREST_6`).
  Exits 3 to 6 (`ENTR_LOST_WOODS_*`) are referenced by no polygon.
- **`En_Skj`** (1,694 lines): five placements.
  - Type 0, room 9: the Skull Kid on the stump. Before `ITEMGETINF_16` he flees within 200. With
    Link on the small stump (type 5) and the ocarina out, he checks Saria's Song: a piece of heart
    (0x10BB, `GI_HEART_PIECE`, `ITEMGETINF_16`). After it: his texts and the Skull Mask trade
    (masks, BACKLOG #39). Hit, he fights with needles (`En_Skjneedle`, 119 lines).
  - Types 1 and 2 with the stump (type 6), room 1: **the memory game** (BACKLOG #47). It needs
    the ocarina's memory game (`AudioOcarina_MemoryGameInit`, `_NextNote`: about 45 lines, logging
    stubs today) and the message box's `MSGMODE_MEMORY_GAME_*` modes (`Message_UpdateOcarinaMemoryGame`
    and its draw cases: about 120 lines; only the start mode is defined in the port). Rewards by
    round (`sOcarinaGameRewards`): rupees and a piece of heart.
  - Other types need the poacher's saw (adult).
- **`En_Dnt_Nomal`** (931 lines), room 1, params 0xFFFF: the **target game** (`ENDNTNOMAL_TARGET`):
  three slingshot bullseyes, then a scrub comes up and gives the bigger seed bag through
  `En_Ex_Item` (540 lines, `ITEMGETINF_1D`). Also needs `EffectSsExtra` (the score, 98 lines) and
  one-point cutscene 4140.
- **`En_Okuta`** (738 lines), room 3: **never spawns.** `object_okuta` is in no Lost Woods room's
  object list, and `Actor_Spawn` refuses an actor whose object isn't loaded. Nothing to port for
  this phase.

**The Sacred Forest Meadow** (child layers 0 and 1):
- **Ported already:** the Mad Scrubs (`En_Dekunuts`, six), the gossip stones, the warp pad
  (`Bg_Mjin`), the grottos' holes, the wonder items.
- **`En_Wf`** (1,454 lines), a gray Wolfos (params 0x1F00) by the entrance. It rises out of the
  ground when Link is within 240 (distance only, `EnWf_WaitAppear`), circles, blocks and backflips
  from his sword (`func_800354B4`, not ported); its death sets switch 0x1F.
- **`Bg_Spot05_Soko`** (114 lines): type 0, four child-only blocks; type 1, the gate that sinks
  on switch 0x1F (with `OnePointCutscene_Attention`, ported).
- **`En_Sa`** (860 lines): Saria. In the meadow with Zelda's letter and without her song, she
  starts `gMeadowSariasSongCs` herself once Link is past z -2220 (`EnSa_AwaitTeachingSariasSong`);
  the cutscene's texts teach and check the song, and `EnSa_BecomeIdle` gives it
  (`Item_Give(ITEM_SONG_SARIA)`). The cutscene has no terminator (its `MISC` ends it); Navi is
  its cue 8. With the song she plays her ocarina (0x10AD).
  - **The same actor is Kokiri Forest's Saria**, left over from Phase 3: on a new save her init
    starts `gKokiriForestSariaGreetingCs` as soon as Kokiri Forest's room 0 loads (Link's first
    walk out of his house), and its end sets `INFTABLE_SARIA_GREETED_LINK`; then she stands by
    Link's house, and in her house (`kokiri_home5`) with the ocarina. **Porting her changes the
    Kokiri runs:** about twenty test files and the new-file playthrough enter room 0 on a new
    save, where the greeting would now play.

**Lon Lon Ranch** (child layers 0 and 1, the stable, Talon's house):
- **The intro already plays:** `gLonLonRanchIntroCs` comes from the entrance table
  (`ENTR_LON_LON_RANCH_0`, both ages, `EVENTCHKINF_B4`); a sandbox entry shows its title card.
- **Malon (`En_Ma1`) is ported whole**, her Epona's Song lesson too: the playback's end reloads
  the ranch with `CS_INDEX_1`, which is layer 5 and `gLonLonRanchEponasSongCs`. **Its terminator
  isn't:** `CS_DEST_LON_LON_RANCH_FROM_EPONAS_SONG` (94: `ENTR_LON_LON_RANCH_1`, a white fade) has
  no case in the port's `cutscene_command_terminator`. Kaepora Gaebora is the cutscene's cue 7
  (`En_Owl`, ported).
- **`En_Horse_Link_Child`** (638 lines): the young Epona, from Talon's return. She runs from Link
  or back home, and after Epona's Song (`R_EPONAS_SONG_PLAYED`, which the port keeps) faces him or
  runs to him. **No `En_Horse` needed** (BACKLOG #51's horse is the adult's). A skin skeleton;
  needs `Horse_RotateToPoint` (`z_horse.c`).
- **`En_Horse_Normal`** (729 lines): the paddock's horses by day (one gallops path 0, the others
  stand or wander), the stable's by night. Skin skeletons; they never notice Link.
- **`En_In`** (1,059 lines): Ingo, by day in the ranch until Talon returns, then in the stable;
  for a child only his texts and his head (the race, about 480 lines, is the adult's).
- **`En_Crow`** (527 lines): fifteen Guays at night (layer 1), diving at Link within 300 after
  100 frames, respawning 300 frames after a kill; every tenth is a big one with a red rupee.
- **`En_Ma2`** (428 lines): adult Malon; `EnMa2_GetSpawnIndex` kills her for a child.
- **Talon's house** (`souko`, BACKLOG #44): `En_Ta`'s ranch house by day, about 600 lines: the
  milk (30 rupees, the bottle, `ITEMGETINF_TALON_BOTTLE`) and the **Super Cucco game** (find his
  three among ten cuccos in 30 seconds, `EVENTINF_CUCCO_GAME_WON`). It needs the HUD timer
  (`Interface_SetTimer` and the timers' part of `Interface_Draw`, about 450 lines with the sub
  timer; not ported) and `func_800F5ACC` (the mini-game music). The cuccos (`En_Niw` 0xD and 0xE)
  are ported.
- **The draw configs** `SDC_LON_LON_RANCH` and `SDC_LON_LON_BUILDINGS`: a window or entrance
  texture by `nightFlag`, as Kakariko's (ADR 0064's `segment_textures`).

### Things checked that the notes asked about

- **GAME-06's ports:** the Lost Woods' bridge cutscene and `Demo_Sa`'s bridge parts are done; the
  other `Demo_Sa` types (the sages', the credits'; BACKLOG #28) aren't in this phase's scenes.
- **BACKLOG #29** (planting beans): with the bean seller (Zora's River, Phase 11), as logged.
- **BACKLOG #51:** the child's Epona needs no `En_Horse`.
- **The Deku scrubs' theatre** in the Lost Woods is a grotto (`En_Dnt_Demo`, `kakusiana`): Phase 20.
- **No intro is missing:** the ranch's plays; the woods and the meadow have none.

### Sizes

| Part | Lines of C (about) |
|---|---|
| `En_Sa` | 860 |
| `En_Wf` | 1,454 |
| `Bg_Spot05_Soko` | 114 |
| `En_Skj`, `En_Skjneedle` | 1,694 + 119 |
| The ocarina's memory game (message box and audio) | 165 |
| `En_Dnt_Nomal`, `En_Ex_Item`, `EffectSsExtra`, one-point 4140 | 931 + 540 + 98 + 40 |
| `En_Horse_Link_Child`, `En_Horse_Normal`, `z_horse.c`'s helper | 638 + 729 + 20 |
| `En_In` (the child's part; the race later) | 580 |
| `En_Crow` | 527 |
| `En_Ma2` (init's checks only) | 30 |
| `En_Ta`'s ranch house and the Super Cucco game, the HUD timer | 600 + 450 |
| Destination 94, two draw configs | 30 |

About 9,000 lines with the games, about 6,300 without the target game and the cucco game.

## The proposal

Three milestones, the exit first:

| # | Milestone | What | Run |
|---|---|---|---|
| 1 | The meadow and Saria's Song | `En_Sa` whole (the meadow, Kokiri Forest's greeting and her house, with the Kokiri runs brought up to date), `En_Wf`, `Bg_Spot05_Soko`; the Lost Woods' maze walked | Kokiri Forest through the woods' right turns to the meadow, the Wolfos killed, the gate, the maze, Saria's lesson: the song learned (the phase's exit) |
| 2 | The Lost Woods' Skull Kids and scrubs | `En_Skj` whole with `En_Skjneedle`, the ocarina's memory game (#47's Skull Kid part), `En_Dnt_Nomal` with `En_Ex_Item`, `EffectSsExtra`, one-point 4140 | The memory game won; Saria's Song on the stump (a piece of heart); the target's three bullseyes |
| 3 | Lon Lon Ranch | Destination 94, the two draw configs, `En_Horse_Link_Child`, `En_Horse_Normal`, `En_In` (the child's), `En_Crow`, `En_Ma2`'s init, `En_Ta`'s ranch house with the milk and the Super Cucco game and the HUD timer (#44) | Malon's lesson, Epona's Song's cutscene back to the ranch, Epona called with the song; the cucco game won |

The scope rule as in GAME-07: every actor whole, except those a child's init kills (their init's
checks, then a placeholder).

## The user's calls

2026-10-14: the split as proposed (three milestones, the exit first); all the games and shops in
(the memory game, the target scrub, Talon's milk and cucco game with the HUD timer); the scope
rule kept; the three coverage table corrections applied with milestone 1's coverage refresh.

## Milestone 1: The meadow and Saria's Song

**The answer:** child Link with Zelda's letter walks from the Lost Woods' entrance through its
right turns, answers the owl, kills the meadow's Wolfos (its death sinks the gate), gets through
the maze past its Mad Scrubs, and climbs to Saria, who teaches Saria's Song. Scripts:
`scripts\run\test-meadow.bat`, `game-meadow.bat`, `sandbox-meadow.bat` (menu 122 to 124).

### What was built

- **Shared code** (ADR 0067): `Actor_FindNearby`, `Actor_IsLockedOn`, `Actor_OtherIsLockedOn`,
  `Actor_TestFloorInDirection`, `func_800354B4`, `func_800355B8`, `func_800359B8`
  (`oot_game::actor_ctx`); `CollisionCheck_CylSideVsLineSeg` (`oot_game::collision_check`);
  `Actor_GetProjectileActor` (the `oot_actors` crate root, with the C's hookshot-timer @bug);
  `EffectSsKiraKira` whole (`oot_game::effect::kirakira`); `Actor_UpdateAlphaByDistance`'s
  cutscene branch (`oot_game::npc`, now shared by Mido and Saria).
- **`En_Sa`** whole (a helper): every type (the greeting, standing in Kokiri Forest and in her
  house, waiting and playing in the meadow), her texts and talk, her seven gesture groups, her
  eyes and mouth, her cutscene cues, the song given at the meadow script's end; four bakes.
- **`En_Wf`** whole (a helper): both Wolfos, every action (buried, idle, circling, running at Link,
  slashing, blocking, the backflips, stunned, hurt, dying), the ×4 hits on its tail or from
  behind, its switch flag at death, the White Wolfos's mini-boss music; six bakes.
- **`Bg_Spot05_Soko`** whole: the child-only blocks and the gate.
- **Saria's greeting everywhere:** `Task::Exit`'s settle presses A through a text; the tests'
  `common::after_sarias_greeting`; `save::left_the_house` in the presets; 21 test files brought up
  to date (a helper), and the route tests expect the greeting's text and script.
- **`Route::Meadow`** with the tasks `FightWolfos` and `ChaseScrub`, the steps `Owl`,
  `MeadowEntered`, `WolfosKilled`, `SariasSongLesson`, `SariasSongPlayed`, `SariasSongLearned`.
- **Coverage:** the `SCENES` and `ACTORS` corrections; Phase 9 in progress.

### Results

- `tests/meadow.rs`: the gate and the blocks from the C (the gate's sink: 0.5 × 1.5 each frame, 11
  frames to 120 down), and the exit's run: 6,853 frames, Link ends with 24 of 48 (three nuts on
  the ramp). `tests/en_sa.rs` (6), `tests/en_wf.rs` (9, a child's fight: the Wolfos dead at frame
  1,670 with no hit taken).
- 1,014 tests (one ignored) pass; `cargo test -p layering` passes; no warnings.
- Goldens: 10 traces changed with Saria and the presets' flag, each proven (`golden/README.md`);
  5 new cases; 193 hashes, 147 cases (`out/golden_base18`; `out/golden_base19` after the fix
  below).

### Decisions

- Saria's greeting plays wherever the game plays it; tests about something else carry its flag
  (ADR 0067).
- The scrubs: bounced and chased where the shield sends their nuts back; run past on the ramp and
  in the dip (BACKLOG #64).

### Corrections to the survey

- Saria's unused-layer placements (Kokiri Forest's 8 and 9) are in room 1, not room 0.
- The Mad Scrubs are the maze's real obstacle for a run (six along the way); the survey didn't
  weigh them.

### Known gaps

- Saria's and the Skull Kid's mask reactions: no masks yet (BACKLOG #39).
- The Wolfos: bombs (`En_Bom`, #33) and the hookshot aren't there for it to react to.
- A Mad Scrub's nut on the ramp and from the dip (#64).
- The Lost Woods' Skull Kids, target scrub and their games: milestone 2.

### Fixes found while building

- `sandbox-graveyard.bat` passed a literal `SHOTS` to `--shots-at`: real frames now.
- `sfx_route`'s Navi check predicted her vanish sound from the silence count after the frame, not
  before (only wrong right after a cutscene: Saria's greeting).
- `zcamera`'s placeholder test had no placeholder left in Kokiri Forest's room 0 (Saria was the
  last): it spawns its own now.
- `En_Encount1`'s and `Obj_Syokudai`'s docs said `En_Wf` and `En_Arrow` weren't ported.

### Fixes after the hand test

- **The Lost Woods' log openings were the fog's colour, black in the game** (the user's report).
  `En_Holl` type 0 (`ENHOLL_H_VISIBLE_NARROW`, the woods' room planes, one in each log) draws a
  black plane (`EnHoll_Draw`: `sPlaneDL`, prim (0, 0, 0, `planeAlpha`)), opaque until Link is near
  and fading as he passes through. Its mesh is the overlay's, which the port hadn't baked, so it
  computed `planeAlpha` and drew nothing. Now baked from `ovl_En_Holl` with `SETUPDL_37` (new) and
  `SETUPDL_0`, and drawn. The royal tomb's corridor has one too. Two goldens changed, proven
  (`golden/README.md`).

### How to check

```
scripts\run\test-meadow.bat
cargo test --release --workspace
cargo test -p layering
python scripts\golden.py check --bin-dir target\game26\release
scripts\run\sandbox-meadow.bat
```

### By hand

1. `game-meadow`: in the Lost Woods, east, north (the owl talks: answer no), east, north, north,
   west, north. Saria's Song gets louder the right way; a wrong corridor ends back in Kokiri Forest.
2. `game-meadow meadow`: walk north; the Wolfos rises within a few steps. Z on it, R to guard, B
   when it's open (after its slash, when it's hurt): it dies with a howl, and the gate ahead sinks
   with the camera turning to it.
3. In the maze, a Mad Scrub popping up: face it and hold R; its nut bounces back and it runs.
   Chase it and slash it.
4. Up the ramp at the north end (the last two scrubs spit at you), past the pillars: Saria's lesson
   starts; play the song back as the staff shows it. At the end Saria plays on her stump; talk to
   her (0x10AD).
5. `game-meadow saria`: on Saria's plateau, the lesson at once.
6. `game-meadow kokiri`: a new save in Link's house: walk out onto the porch; Saria greets Link
   ("Hi, Link!"), then stands by the ladder. In her house later (Kokiri Forest's north-west), she
   stands inside with the ocarina.

