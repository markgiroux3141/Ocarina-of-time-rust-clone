# GAME-08: The Lost Woods, the Sacred Forest Meadow and Lon Lon Ranch

**Goal:** Phase 9 of the [roadmap](ROADMAP.md): child Link with Zelda's letter in the Lost Woods,
through the Sacred Forest Meadow's maze to Saria and her song, and at Lon Lon Ranch for Malon,
Epona's Song and the child's Epona, Ingo, and Talon's house. Exit: Saria's Song learned.

**Status:** done (2026-10-15; its exit review at the end). Decided 2026-10-14; the user chose:
- the three milestones below, the exit first;
- every game and shop the phase's scenes hold: the Skull Kids' memory game, the Lost Woods' target
  scrub and its seed bag, Talon's milk and Super Cucco game (with the HUD timer);
- GAME-07's scope rule: every actor whole, except those a child's init kills (their init's checks,
  then a placeholder);
- the coverage table's corrections below.

| # | Milestone | Status |
|---|---|---|
| 1 | The meadow and Saria's Song: `En_Sa` (with Kokiri Forest's greeting), `En_Wf`, `Bg_Spot05_Soko`, the woods' maze walked | done (2026-10-14) |
| 2 | The Lost Woods' Skull Kids and scrubs: `En_Skj`, the memory game, the target scrub | done (2026-10-14) |
| 3 | Lon Lon Ranch: Epona's Song's end, the foal and the horses, Ingo, the Guays, Talon's house and the cucco game | done (step (a) 2026-10-14, step (b) 2026-10-15) |

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


## Milestone 2: The Lost Woods' Skull Kids and scrubs

**The answer:** child Link with Saria's Song and the slingshot plays the song to the Skull Kid on
his stump (a piece of heart), shoots the target scrub's bullseye three times in a row (the bigger
seed bag), and wins the Skull Kids' memory game three rounds running (a green rupee, a blue
rupee, a piece of heart). Scripts: `scripts\run\test-lost-woods.bat`, `game-lost-woods.bat`,
`sandbox-lost-woods.bat` (menu 126 to 128; 125 is the glb export's, ADR 0068).

### What was built

- **Shared code** (ADR 0069): the ocarina's memory game (`AudioOcarina_MemoryGameInit`,
  `_MemoryGameNextNote`, with the C's read of the song before it as an @bug), the message box's
  `MSGMODE_MEMORY_GAME_*` modes and `Message_UpdateOcarinaMemoryGame`, the metronome's sounds;
  one-point cutscene 4140; `Effect_Ss_Extra` (the score) and `interfaceCtx.unk_23C`;
  `SETUPDL_41` (moved from the helper into `gbi::setup_dl`).
- **`En_Skj`** and **`En_Skjneedle`** whole (a helper): the stump Skull Kid (his flight before
  `ITEMGETINF_16`, Saria's Song on the small stump and its heart piece, his texts after, the
  Skull Mask trade, the fight: needles, hits, the backflip, death and its orange rupee), the
  memory game's two Skull Kids and its stump (the rounds, `sOcarinaGameRewards`, the failures,
  `ITEMGETINF_17`), the adult Skull Kids' init; two bakes (opaque and fading).
- **`En_Dnt_Nomal`** and **`En_Ex_Item`** whole (a helper): the target scrub (the bullseye and
  the ring, the count, the camera, its walk, talk and spit, its burrow back) and the mask
  contest's audience (driven by `En_Dnt_Demo`'s signals, Phase 20); every prize of `En_Ex_Item`
  (the bowling alley's, the treasure chest game's rupees and key, the Great Fairy's magic, the seed
  bag); five bakes.
- **The `meadow-saria` preset**, from a diff of the save over milestone 1's run.
- **`Route::LostWoods`** with the tasks `ShootUntil` and `MemoryGame`, the steps `SkullKidSong`,
  `SkullKidHeart`, `Bullseye`, `BulletBag`, `MemoryGame`, `MemoryRound`, `MemoryGameWon`.
- **Coverage:** `En_Okuta` to Phase 11; the glb export's characters gain the Skull Kid and the
  two scrubs (`Deku Scrub`, `Deku Scrub (theatre)`).

### Results

- `tests/lost_woods.rs`, the exit's run: 5,736 frames. Saria's Song on the stump at frame 319,
  the heart at 664; the bullseyes at 1,085, 1,099 and 1,113, the seed bag at 1,625; the memory
  game from 1,812, its 13 turns (3 to 5 notes, 3 to 6, 3 to 8), won at 5,736. Link ends with two
  more pieces of heart, the 40 seed bag full and six more rupees, and takes no hit.
- `tests/en_skj.rs` (16), `tests/en_dnt_nomal.rs` (8), `tests/en_ex_item.rs` (9), and two unit
  tests of the memory game's notes in `oot_game`.
- 1,054 tests (one ignored) pass; `cargo test -p layering` passes; no warnings.
- Goldens: the five `meadow` hashes changed, proven (`golden/README.md`): the target scrub's
  blinks call `Rand` from the scene's start, and the owl's camera in room 2 picks the other of its
  two. Five new cases (`lost_woods`, `_stump`, `_target`, `_scrub`, `_game`). 199 hashes, 152
  cases (`out/golden_base20`).

### Decisions

- The route takes the woods' rooms in the order that ends in the hollow: room 9's stump, the
  target from room 1's clearing, then down into the hollow for the memory game.
- The preset adds the slingshot (a player has it after the Deku Tree); the earlier presets stay
  as they are.
- `En_Okuta` is left out, as the survey found; the coverage table moves it to Zora's River's phase.

### Corrections to the survey

- The memory game's turns: each round restarts at three notes, so the rounds are three, four and
  six turns, not one each.
- Room 1's clearing isn't open onto the hollow: a wall 25 high runs along its edge but at its
  north end, where Link drops down. The memory game's stump stands 40 above the hollow's floor.
- `func_80033C30` (the stage scrub's draw) is a circle shadow, not dust; `object_dnk` is loaded in
  every Lost Woods room (the stage scrub's object), though only the grotto places it.
- `Effect_Ss_Extra` is effect 0x1E (I first wrote 0x24, `Ice_Smoke`'s; checked against
  `effect_ss_table.h`).

### Known gaps

- No masks (BACKLOG #39): the Skull Mask trade is reached only in a test.
- A fading Skull Kid draws as one translucent pass (BACKLOG #67).
- `En_Ex_Item`'s highlights (`func_8002EBCC`, `func_8002ED80`) aren't drawn, as for the shop's
  items (the roadmap's rendering debt).
- The stage scrubs wait on `En_Dnt_Demo` (Phase 20).
- The sandbox has no `--room` (BACKLOG #65); `RAD_TO_BINANG`'s precision (#66).

### Fixes found while building

- The preset first gave Epona's Song (item 0x61) for Saria's (0x62): the route's song check on
  the stump never matched. Checked against `item.h`.
- `Task::Shoot` now ends on a condition (`ShootUntil`), the switch flag being one.

### How to check

```
scripts\run\test-lost-woods.bat
cargo test --release --workspace
cargo test -p layering
python scripts\golden.py check --bin-dir target\game26\release
scripts\run\sandbox-lost-woods.bat
```

### By hand

1. `game-lost-woods`: go left at the first fork (west). The Skull Kid on his stump backflips away if
   you come within 200 of him; stand on the small stump in front of him instead, take out the
   ocarina (K) and play Saria's Song (K L J K L J): he plays along on his flute, says "You know
   Saria's Song", and gives a piece of heart. Talk to him again after.
2. Back east through the first room and right (east) at the fork: the target across the hollow.
   C-Right (L) for the slingshot, aim at the target's middle and let go: "100" over the scrub's
   flower on a bullseye, "30" on the ring (which starts the count again). Three bullseyes in a
   row: the camera turns to the scrub, it comes up, walks over, talks, spits a seed bag at you.
3. `game-lost-woods target`: the same from the clearing's edge.
4. In the clearing, its north-east corner: drop into the hollow and climb onto the stump between
   the two Skull Kids (they fade in). Take out the ocarina: they play notes (a metronome ticks),
   then it's your turn: play them back. A wrong note or too slow (160 frames): he says so and offers another try.
   Three rounds: a green rupee, a blue rupee, then a piece of heart.
5. `game-lost-woods game`: on the memory game's stump at once.
6. Hit the stump Skull Kid with the sword after his heart piece: he backflips away.


## Milestone 3: Lon Lon Ranch

The user's call (2026-10-14): two steps. (a) The ranch's story: Epona's Song's end, the draw
configs, the foal, the horses, Ingo, the Guays, adult Malon's init. (b) Talon's house: the milk,
the Super Cucco game, the HUD timer. Each with its own section here, goldens and commit.

### Step (a): the ranch's story

**The answer:** child Link with the Lost Woods' games won comes into Lon Lon Ranch by its gate
(its intro plays), walks through the corral's gate to Malon by the foal, talks to her three
times, takes out the ocarina and plays Epona's Song back; the song's scene plays and ends back
at the ranch, where the foal comes to him. He walks off; once she has gone home, Epona's Song
calls her to him again. Scripts: `scripts\run\test-ranch.bat`, `game-ranch.bat`,
`sandbox-ranch.bat` (menu 129 to 131).

#### What was built

- **Shared code** (ADR 0070): `oot_game::horse` (`Horse_RotateToPoint`); `skin::get_limb_pos`
  (`Skin_GetLimbPos`); the draw configs `SDC_LON_LON_RANCH` (the ranch's windows, segment 8, OPA)
  and `SDC_LON_LON_BUILDINGS` (Talon's house's entrance, segment 8, XLU) by `nightFlag`; the HUD
  timers' save fields and `Interface_SetTimer` (the countdown and the draw are step (b)'s);
  `QUEST_SONG_EPONA`.
- **`En_Horse_Link_Child`** and **`En_Horse_Normal`** whole (a helper): the foal (killed before
  Talon's return; idling, then running from Link or home; turning to Link on Epona's Song and
  coming to him; her eyes, sounds, colliders) and the horses (the paddock's by rot.z, age, day and
  flags, the galloper round path 0, the wanderers, the stable's by night with their clones and the
  clones' shadows; the Gerudo Fortress's). Three bakes.
- **`En_In`** (a helper): the child's Ingo whole (working in the ranch by day before Talon's
  return, in the stable after it by day and by night, his texts 0x203F/0x2040 and 0x2045/0x2046
  and their flags, his head and chest turned to Link, the basket and pitchfork), the adult's start
  modes decided whole; the adult's race and rental a placeholder (Phase 13). Ten bakes.
- **`En_Ma2`**: `EnMa2_GetSpawnIndex` whole (killed for a child), a placeholder for the adult's.
- **`En_Crow`** whole (a helper): the Guays' flight, dive, the turn-away from a nut, death,
  drops and respawn, the tenth death back big with a red rupee. One bake.
- **The `lost-woods-games` preset**, from a diff of the save over milestone 2's run.
- **`Route::Ranch`** with the task `OcarinaOut` (the ocarina out, no song) and the steps
  `EponasSongPlayed`, `EponasSongCs`, `RanchFromSong`, `FoalCame`, `FoalHome`, `EponasSong`,
  `FoalCalled`.
- **The glb export:** Ingo and the Guay (`characters.rs`); the horses were there.
- **Coverage:** Phase 9's ten actors all ported; the ranch's and Talon's house's draw configs.

#### Results

- `tests/ranch.rs`, the exit's run: 3,073 frames. Malon's texts at 890, 1,233 and 1,325, the
  lesson's 0x2061 at 1,422, the playback done at 1,760, layer 5 at 1,928 (its terminator at its
  frame 300), back by `ENTR_LON_LON_RANCH_1` at 2,272, the foal at Link at 2,282; home at 2,793,
  the song at 2,838, at Link again at 3,073. Epona's Song given, the intro's `EVENTCHKINF_B4` and
  Malon's three flags set, no hit taken.
- `tests/en_horse_link_child.rs` (8), `en_horse_normal.rs` (6), `en_in.rs` (8), `en_ma2.rs` (2),
  `en_crow.rs` (8); unit tests of `Horse_RotateToPoint`, `Skin_GetLimbPos` and two of `En_In`'s;
  the draw configs against the C interpreter and the rooms' textures.
- 1,091 tests (one ignored) pass; `cargo test -p layering` passes; no warnings.
- Goldens: none changed (199 of 199); 7 new cases (`ranch`, `_malon`, `_song`, `_foal`, `_guays`,
  `_ingo`, `stable_night`). 208 hashes, 159 cases (`out/golden_base21`).

#### Decisions

- Ingo's adult part is a placeholder chosen in his init (the C chooses on his first update), as
  `En_Ta`'s ranch house was (ADR 0070).
- The foal's eyes are swapped per draw, not baked per eye: the skin's animated limb loads its
  vertices through the eye's segment first.
- The route answers "the foal called with the song" from her home: after the cutscene she comes by
  herself (`ENTR_LON_LON_RANCH_1`), so Link waits out her 300 frames and plays it from 450 away.

#### Corrections to the survey

- Destination 94 was already in the port's terminator (`TERMINATOR_DESTINATIONS`): nothing to add.
- The stable is `SDC_DEFAULT`; only the ranch and Talon's house have the two configs.
- The Guays' sphere grows to 59, not 60 (`20 × 0.03f × 100.0f` truncates), and it's the Guay that
  dies tenth that comes back big, not the next to die.
- The foal circles Link near her home: `func_80A69F5C` turns by Link's facing ± 0x4000, not by the
  difference from her own yaw (@bug (game)).
- `En_In`'s adult-era head is never drawn (its override skips the head); the obedient Ingo with
  Epona keeps the child-era clothes (only `params > 0` swaps them).
- The ranch's night layer places no horses; the Gerudo Fortress's horses are on cutscene layers only.

#### Known gaps

- Talon's house by day (the milk, the Super Cucco game, the timers' countdown and draw): step (b),
  BACKLOG #44.
- The horses' sounds at their own points (#68); `ActorShadow_DrawHorse` (the shadows' debt).
- The adult's Ingo, Malon and race (Phase 13); masks (#39).

#### Fixes found while building

- None outside the new code.

#### How to check

```
scripts\run\test-ranch.bat
cargo test --release --workspace
cargo test -p layering
python scripts\golden.py check --bin-dir target\game26\release
scripts\run\sandbox-ranch.bat
```

#### By hand

1. `game-ranch`: the ranch's intro, then south down the lane between the house and the stable. In
   the open, the corral: its gate is in the near fence, to the right. Inside, Malon sings by
   the foal; the horses wander and one gallops round the track.
2. Talk to Malon three times (0x2047, 0x2048, 0x2049). Take out the ocarina by her (K): she plays
   Epona's Song; play it back (I J L I J L). The song's scene (the camera rises over the corral),
   then back by the foal: she comes to you and follows you about (she may nudge you along).
3. Walk away across the corral; after about 15 seconds she trots home. Play Epona's Song: she
   neighs and comes to you.
4. Near her home she runs circles round you: the game's own (the C turns her by your facing).
5. `game-ranch night`: the Guays circle overhead and dive at you within 300 (half a heart). Cut one
   down: it falls, shrinks, drops something, and is back 15 seconds later. Every tenth kill comes
   back three times the size and drops a red rupee.
6. `game-ranch stable`: Ingo working (pitchfork, basket); talk to him (0x2045, then 0x2046).
   `game-ranch stable-night`: two horses in their stalls and two more opposite (the clones), the
   rails hiding their middles as the painted rails do.
7. `game-ranch ingo`: before Talon's return, Ingo working by the gate; 0x203F then 0x2040.

### Step (b): Talon's house

**The answer:** after the foal, Link walks back out of the corral and up the gate's lane to
Talon's house, talks to Talon at his table and plays the Super Cucco game: Talon throws his three
cuccos among the house's ten, the timer counts down from 30 at the top left, and Link brings each
super cucco to him; won, he sends Link back in by a white starburst and gives him a bottle of milk.
The scripts are step (a)'s, with Talon's and the timers' tests and a `house` start.

#### What was built

- **The HUD's timers** (ADR 0071, `oot_game::timer`): `Interface_SetTimer`, `_SetSubTimer`,
  `_SetSubTimerToFinalSecond`; `Interface_Draw`'s state machine for the main timer (down, up, the
  hot rooms' and the water's, the stop and the freeze) and the sub timer (the trade quest's respawn,
  the marathon), the beeps, the digits and the draw (the clock, five digits, red under 10 seconds);
  `Interface_Update`'s hazard timer; `play->damagePlayer` as a hook `Player_Init` sets. The callers
  brought up to date: Player's item buttons, `Bg_Relay_Objects`, `Demo_Kankyo`, Ingo.
- **`En_Ta`'s ranch house** whole (a helper): the day's init with the three super cuccos, the talks
  (0x207E, 0x207F, 0x208B) and their choices, the milk (30 rupees, an empty bottle), the Super Cucco
  game (the throw, the timer and its music, the hunt with the 7-frame hold, 0x2082, 0x2083, 0x2084
  won, 0x2081 time's up with the floor camera), the starburst back to `ENTR_LON_LON_BUILDINGS_2`,
  the prizes (0x2086 and the bottle of milk the first time, 0x2088 and milk after, 0x2085 a retry for
  5 rupees), the destroy. BACKLOG #44 closed.
- **`Route::Ranch` goes on:** `Task::OpenDoor` (an `En_Door` opened: the first route through one),
  `Task::CatchSuperCucco`, and the steps `TalonsHouse`, `CuccoGame`, `SuperCucco`, `CuccoGameOver`,
  `MilkBottle`.
- **Sounds:** `NA_SE_SY_MESSAGE_WOMAN`, `_WARNING_COUNT_N`, `_WARNING_COUNT_E`, `_FOUND` in
  `audio::sfx`; the HUD's clock and the colon's sprites.

#### Results

- `tests/ranch.rs`, the whole run: 5,792 frames. Step (a)'s run to frame 3,073 (unchanged, frame for frame); into Talon's house
  at 3,585; the game begun at 4,490 (0x207E answered yes, 0x2080, the throw), the three super
  cuccos at 4,621, 4,810 and 4,981 (0x2082, 0x2083, 0x2084: won; the timer read 00:25 at frame 4,700), back by
  Talon at 5,010, 0x2086, 0x2087 and the bottle of milk (0x98) at 5,792. Ten rupees paid,
  `ITEMGETINF_TALON_BOTTLE`, the milk in the first bottle's slot, the game's flags cleared again.
- `tests/en_ta_house.rs` (10: by day and night, before Talon's return, the talks and their
  choices, the game won through the real lifts and the reload, a first win set up directly, later
  wins and the milk, a loss's retry, time running out with the floor camera); `tests/timer.rs` (4:
  the countdown's frames, the stop, counting up and the sub timer, the hazard timer); a unit test of
  the slide.
- 1,106 tests (one ignored) pass; `cargo test -p layering` passes; no warnings.
- Goldens: `ranch`'s trace and shot changed (the route goes on; its first 3,073 frames equal step
  (a)'s), 206 the same; 2 new cases (`ranch_cucco_game`, `ranch_milk`). 210 hashes, 161 cases
  (`out/golden_base22`).

#### Decisions

- The timers' state machine runs in the game frame where `Play_Draw` would run `Interface_Draw`,
  and their statics outlive the play state (ADR 0071).
- `play->damagePlayer` is a hook on the play state, as in the C, rather than a call across crates.
- The route catches the super cuccos nearest first, picking them by gravity (a found one floats off
  with 0.1), not by Talon's own list.

#### Corrections to the survey

- `Interface_Draw`'s timer block is the state machine as well as the draw; the timer is hidden
  whenever a text is open (the draw is inside the same checks).
- The digits' rectangle is `VREG(42)` (250) tall; with no mask the tile clamps, so only the digit's
  16 rows show (scaled by 880/1024).
- `ENTR_LON_LON_BUILDINGS_0` is the door into Talon's room (room 1); `_1` is the cows' room.

#### Known gaps

- The mask texts (0x7178 to 0x717B: no masks, #39); the shared helpers' private copies (#69).
- `minigameState` and `shootingGalleryStatus` read 0 (the horseback archery and the shooting
  galleries aren't ported).

#### Fixes found while building

- None outside the new code.

#### How to check

```
scripts\run\test-ranch.bat
cargo test --release --workspace
cargo test -p layering
python scripts\golden.py check --bin-dir target\game26\release
scripts\run\sandbox-ranch.bat
```

#### By hand

1. `game-ranch house`: Talon dozes at his table, two cuccos on it and one at his feet. Talk to him:
   0x207E, answer yes (10 rupees). He throws his three cuccos; the timer shows 00:30 in the
   middle, slides under the hearts and counts down.
2. Find his three among the cuccos (they look the same: they're the ones he threw), lift one (A),
   and keep holding it: Talon speaks (0x2082) and it flies off; then 0x2083; the third ends the
   game (0x2084, a fanfare, a white starburst). At 10 seconds the music speeds up; at 0, 0x2081
   with the camera on the floor, and a black starburst.
3. Won: Talon speaks at once (0x2086 and its question, either answer, then 0x2087), then gives a
   bottle of milk.
   Talk again: 0x208B (milk for 30 rupees with an empty bottle, or play again).
4. Lost: 0x2085, a retry for 5 rupees.


## Phase 9's end (the exit review, 2026-10-15)

**The exit, met:** Saria's Song learned (milestone 1, `Route::Meadow`). The phase's other places
are played through too: the Lost Woods' Skull Kids and scrubs (milestone 2), Lon Lon Ranch, its
foal and Talon's house (milestone 3). Commits: the actor schema (part A, 05f4f0e), 12c0d60, da59e44,
bd4bbf5 and 53fda8c; the glb export (9f413b4) between milestones 1 and 2.

### What the phase built

- **Actors, whole:** `En_Sa`, `En_Wf`, `Bg_Spot05_Soko`, `En_Skj`, `En_Skjneedle`, `En_Dnt_Nomal`,
  `En_Ex_Item`, `En_Horse_Link_Child`, `En_Horse_Normal`, `En_Crow`, and `En_Ta`'s ranch house
  (the actor whole now); the child's `En_In` (the adult's race a placeholder, Phase 13) and
  `En_Ma2`'s init. 131 ported actors at the phase's start, 143 at its end; every one of the
  phase's scenes' actors that a child meets is ported (COVERAGE: 10 of 10).
- **Systems:** the actor schema for every actor (ADR 0066); the `z_actor.c` helpers the Wolfos
  needed, `EffectSsKiraKira` (ADR 0067); the ocarina's memory game, `Effect_Ss_Extra`, one-point
  cutscene 4140 (ADR 0069); the first live skin skeletons, the ranch's draw configs (ADR 0070); the
  HUD's timers whole and `play->damagePlayer` (ADR 0071); the characters as .glb (ADR 0068).
- **Runs:** `Route::Meadow` (6,853 frames), `Route::LostWoods` (5,736), `Route::Ranch` (5,792),
  each from the last one's end state as a preset: `graveyard-sun`, `meadow-saria`,
  `lost-woods-games`, and now `ranch-epona` (below).
- **Numbers:** 996 tests at the phase's start (after the schema), 1,106 at its end (one ignored);
  goldens 187 hashes and 142 cases, then 210 and 161 (`out/golden_base22`). Menu entries 122 to
  131.

### Phase 10's start: the `ranch-epona` preset

The save at `Route::Ranch`'s end, found by diffing it over the run, as each milestone did:
`lost-woods-games` and the ranch's intro (`EVENTCHKINF_B4`), Malon's flags
(`EVENTCHKINF_TALKED_TO_CHILD_MALON_AT_RANCH`, `_CAN_LEARN_EPONAS_SONG`,
`INFTABLE_TOLD_EPONA_IS_SCARED`), Epona's Song, Talon's flags (`INFTABLE_TALKED_TO_TALON_IN_RANCH_HOUSE`,
`ITEMGETINF_TALON_BOTTLE`) and the bottle of milk in the first bottle's slot, 16 rupees. No scene
flags change; the clock doesn't move (the ranch's and the house's rooms set no time speed: the
ROM's headers). `tests/ranch.rs` checks the preset against the run's end: the same flags,
inventory, buttons, rupees and every scene's flags.

With it, Link has the Kokiri Sword and Deku Shield, the Kokiri Emerald, three hearts and two
pieces of heart, the Fairy
Ocarina with Zelda's Lullaby, Epona's Song, Saria's Song and the Sun's Song, the slingshot with
the 40 seed bag, Zelda's Letter (shown at Kakariko's gate: the way to Death Mountain is open), and
the bottle of milk.

### Left for later (all in BACKLOG or the phases' scopes)

- **Masks** (#39): the Skull Kid's Skull Mask trade, every mask reaction; the shared helpers'
  private copies (#69) with them.
- **The adult's** (Phase 13): Ingo's race and rental, adult Malon, `En_Horse` (#51).
- **The ocarina's** scarecrow songs and frogs (#47, Phase 11); `En_Okuta` (Phase 11).
- **The Deku theatre** (`En_Dnt_Demo`, Phase 20); the stage scrubs wait on it.
- **Fidelity checks:** a Mad Scrub's nut on the meadow's ramp (#64), `RAD_TO_BINANG`'s precision
  (#66), a fading Skull Kid's draw (#67), the horses' sound positions (#68).
- **Tools:** the sandbox's `--room` (#65).
- **Rendering debt** (ROADMAP's cross-cutting): the shadows (`ActorShadow_DrawHorse` and the
  circle's alpha), `En_Ex_Item`'s highlights.

### Checked

- ROADMAP (Phase 9 done, its milestones' entries), COVERAGE (Phase 9 done, 10 of 10), the ADR
  index (0066 to 0071), BACKLOG (#44 struck; #63 to #69 open) agree with this doc.
- `cargo test --release --workspace` passes (1,106), `cargo test -p layering` passes, the goldens
  check (210 of 210).
