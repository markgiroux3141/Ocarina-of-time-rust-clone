Next piece of work: **Phase 12, milestone 2: the Door of Time to the Chamber of the Sages and out**
(GAME-11, the phase's exit). Build it, run it, write it up, then stop for my hand test. Commit only
when I ask.

**Where things stand**
- **Milestone 1 is done, hand-tested and pushed** (fe527aa, handoff 05b32a5). Read
  `docs/GAME-11-the-temple-of-time-and-adult-link.md`: the survey's route (steps 1 to 7), the
  proposal's row 2, and milestone 1's section (what it built, its corrections, "Found by hand").
  ADR 0088 is the age switch.
- **Milestone 2's actors are already in**, merged early from milestone 1's helper: `En_Rl` (Rauru)
  whole, `Demo_6K` types 14 to 19 (the others placeholders chosen at init), `En_Xc` types 9 and 1
  and the pedestal block (the others placeholders; type 2's actions, SHEIK_ACTION_30 on, are a
  logged no-op for milestone 3). They have unit tests but have never run in their cutscenes.
- **The age switch works both ways** (`linkAgeOnLoad`, `Inventory_SwapAgeEquipment`,
  `Player_Destroy`), and terminators 8, 25 and 18 set what they should. `Bg_Toki_Swd`'s reveal and
  the first pull are ported but no run has played them: milestone 1's runs started with 0x4F and
  0x45 set.
- 1,794 tests pass, 1 ignored. Goldens: 343 hashes, 255 cases, `out/golden_base41`. **Pack format
  33** in `out/data27`; build into `target/game26` (`scripts/run/_env.bat`).
- No hand test is owed (`docs/HAND-TESTS-OWED.md`); if I defer this one, add its row.
- Next numbers: ADR 0089, BACKLOG #110, menu entry 172.

**Milestone 2's scope** (GAME-11's proposal row 2; line numbers are the decomp's):
- **`Scene_DrawConfigChamberOfTheSages`** (`z_scene_table.c:408`, `SDC_CHAMBER_OF_THE_SAGES`), in
  `oot_game::scene_table` with the others.
- **`gSaveContext.chamberCutsceneNum`** (not in the save: `z_common_data.c:30` starts it at
  `CHAMBER_CS_FOREST`, `z_kankyo.c:389` is the debug register's). Set by the destinations 29 to 31
  (`z_demo.c:852-868`; the port's `cutscene.rs:1076` logs it) and `Door_Warp1`
  (`z_door_warp1.c:729, 758`); read by `Demo_Sa`, `Demo_Du`, `Demo_Im` (their notes in
  `demo_sa.rs`, `demo_du.rs`, `demo_im.rs`). Wire the reads and writes the ported code has; the
  medallions' scenes themselves stay with their phases.
- **`Bg_Spot00_Break`** whole (80 lines; the broken drawbridge at Hyrule Field's spawn 7, the
  user's call 5). It closes Phase 12's coverage row.
- **The helper's open points, checked in their real cutscenes:**
  - Rauru's translucent fade (`En_Rl/xlu`, `gActorSetupXluDL` standing in for
    `Gfx_SetupDL_25Xlu`'s render mode): look at the shots.
  - `func_80AE7668` turns Link through `play.actors.actor_mut` (as `Boss_Dodongo` does). Player
    can't be written during another actor's update except through `PlayerIface`: check it works
    in the run, and move it to `PlayerIface` if it doesn't.
  - Rauru's and Sheik's arm limbs in `ootx`'s `characters.rs` are empty (fill them from the
    skeletons); `func_80AE73D8`'s static flag is kept per instance (the C's is per overlay: fine
    while only one Rauru exists, say so in his docs).
- **Things to check** from GAME-11's survey: who loads the Temple's layers 4 and 6 in play (the
  cutscene indexes, before the runs); adult Link in the ruins' prerendered scenes (`shrine_r`,
  `enrui`, `market_ruins`: their fixed and pivot cameras); the ReDeads' range (`en_rd.rs:830`);
  `ENTR_TEMPLE_OF_TIME_4`'s spawn (0, 70, 46), facing -0x0E39, which Sheik's script is placed from.

**The runs** (short, chained by presets; no fights):
- **The door** (from `ocarina-of-time` at `ENTR_TEMPLE_OF_TIME_0`): the Temple's intro
  (`gTempleOfTimeIntroCs`, `EVENTCHKINF_A7`), the Ocarina of Time put on a C button in the pause
  menu (`ocarina-of-time` has it on no button: `Task::OpenMenu`, `Task::EquipOnC`), the Song of
  Time at the altar (`En_Okarina_Tag` 4 at (-1, -40, 1384)), `gDoorOfTimeOpeningCs` (the stones,
  the door sliding, 0x4B, `Door_Toki` off). Its end a preset (`door-of-time`).
- **The first pull to the Chamber** (from `door-of-time`, or a debug start in room 0 if the walk
  through the `En_Holl` is the run's only content): the reveal (`gRevealMasterSwordCs`, 0x4F,
  Navi's 0x70E6 to 0x70E8), the pull (terminator 8 the first time: 0x45, `ENTR_CUTSCENE_MAP_0`
  with `CS_INDEX_3`), Ganondorf (`hiral_demo` layer 7, `En_Viewer` 7, 0x700A; terminator 25: the
  age), the Chamber (layer 4, the blue warp's arrival, Rauru's texts 0x704E to 0x7053 with the
  choice, `Demo_Effect` 0xE, `Item_Give(ITEM_MEDALLION_LIGHT)`, 0x0040, 0x70E9; terminator 18).
  Its end the preset `light-medallion` (the adult's first save state).
- **The awakening** (from `light-medallion` at `ENTR_TEMPLE_OF_TIME_4`): `En_Xc` 9 starts
  `gTempleOfTimeFirstAdultCs` (0xC5), Navi's 0x7054, Sheik's 0x7056, 0x7057, 0x7077 to 0x707A;
  `Demo_6K` 0x10, `Bg_Spot02_Objects` 4; Player's 14 modes, mode 30 (the sword drawn). `Demo_Effect`'s
  shrinking time warp sets 0xC9 after 250 frames. Its end the preset **`adult-awake`, Phase 13's
  start**.
- **The exit** (a debug start from `adult-awake` in room 0): out through the `En_Holl` into room 1,
  exit 1 (`ENTR_TEMPLE_OF_TIME_EXTERIOR_DAY_1`, `shrine_r` for an adult). The run ends outside the
  Temple: the exit. The ruined market (`market_ruins`, eight ReDeads), `enrui` and the field at
  `ENTR_HYRULE_FIELD_7` (`Bg_Spot00_Break`) are `game-*.bat` debug starts for my hand test; a run
  past the ReDeads only if it can go round them without a fight.
- Tests from the C for `SDC_CHAMBER_OF_THE_SAGES`, `chamberCutsceneNum`, `Bg_Spot00_Break`, and the
  runs' steps, texts and flags. Each run's end checked against its preset (`ends_as_preset`).
- A `game-*.bat` with starts for each: the door, the reveal and pull, the Chamber, the
  awakening, outside the Temple, the ruined market, the field by the drawbridge.

**Helpers:** the framework parts here are small (`SDC_CHAMBER_OF_THE_SAGES` 27 lines,
`chamberCutsceneNum`, `Bg_Spot00_Break` 80): keep them in the main session. If there's room, one
helper may start milestone 3's `En_Xc` type 2 (the Prelude, SHEIK_ACTION_30 on) and the harp's
draws. Give it a tight brief, its own worktree under `.claude/worktrees/` on a branch from HEAD,
its own pack copy (`out/data27_<name>`) and target; the rules in `out/handoff_g11m1/HELPER_RULES.md`
(copy them and update the numbers). At most two helpers at once.

**Merging:** `git diff <base>..<branch> > patch`, `git apply --reject`, then the `.rej` files by
hand. The usual ones: `oot_actors/src/lib.rs`'s `pub mod` lines and PROFILES (milestone 1's
rejected hunk was the `pub mod` lines), the editor's `actors.rs`, `ootx`'s `characters.rs`.
`git apply` writes CRLF, so put each file back as it was (`lib.rs`, `characters.rs`, `actors.rs`,
`save.rs`, `item.rs`, `effect/mod.rs`, `player.rs`, `play_scene.rs`, `docs/BACKLOG.md` and the
`.bat` files are CRLF; most others LF; new files LF). Remove the worktree, its branch and its pack
copy after.

**From milestone 1 (applies here)**
- **Debug starts and the story's flags:**
  - A debug start's entrance can draw what the story never shows there. `Demo_Effect` draws the
    pull's time warp at `ENTR_TEMPLE_OF_TIME_4` whatever the flags; the user saw it as a blue ring
    round the pedestal. Enter the way the story does, then move with `--room`/`--at`
    (`Route::start_room`, `start`).
  - A preset must hold every flag the story has by that point, or placed actors act on the gap.
    Without `EVENTCHKINF_55`, `En_Xc` 9 (placed at (0, 71, 11)) stands by the pedestal and pushes
    the arriving adult 27 back. Without 0xC5 she plays the awakening; without 0xC9 the Chamber's
    return warp plays.
- **Tools and their quirks:**
  - The sandbox needs `--child` for a route whose preset makes Link an adult (the route's save
    starts from a child's), and has no `--adult`.
  - Room actors spawn a few frames after `Play_Init` (their objects load): tasks and tests wait for
    them rather than failing.
  - `Door_Toki` and other actors without `ACTOR_FLAG_UPDATE_CULLING_DISABLED` update only inside
    their culling volume: a test places Link near them.
  - `DynaPoly::set_collision_disabled` rebuilds the lookup on every call, and an engine test relies
    on that: guard the call in the actor, don't change the engine.
  - `Task::Pedestal` (towards `Bg_Toki_Swd`, A, until its cutscene starts) and the runs
    `Route::MasterSwordPull` and `Route::MasterSwordBack` are the models for new tasks and routes.
- The adult's equips after the first swap: B the Master Sword, C-Down bombs, C-Right the Ocarina of
  Time (the survey said empty: wrong).

**From Phase 11 (still applies)**
- Settle a way with the real game's walkthroughs when the geometry is unclear (Wikibooks' pages
  fetch; StrategyWiki and ZeldaDungeon refuse).
- Probe floors and water before writing a run. `out/handoff_m9/probe_m9.rs` is the latest probe:
  copy it into `oot_actors/tests`, run with `--ignored` and `PROBE_OUT` set to an absolute path,
  and remove it after.
- A debug start can land beside a forced text: read it first in a run. An entrance with an intro
  cutscene sets its flag in the run (`ends_as_preset` then needs it in the preset, or as an
  `extra`).
- The run tasks: `Task::OcarinaNotes`, `Task::OcarinaHeard`, `Task::TextsFor(Cond, Step)`,
  `Task::WalkUntil`, `Task::DiveFor`, `Task::LiftWho`, `Task::Texts`, `Task::Hurry`,
  `Task::WaitCameras`, `Task::Pedestal`, `Task::NewScene`, `Task::OpenMenu`, `Task::EquipOnC`.
- Diagnostic log lines (silent unless `RUST_LOG` is set): `[target]`, `[camera]`, `[frame]` at
  info, `[sfx] start` at debug; `OOT_PAD_DEBUG=1`. PowerShell's `2>` writes UTF-16: decode before
  grepping.

**Lessons (all still apply)**
- **Fights and routes:** never script an enemy fight or the long story chain, even if a brief
  names one. Give a debug start beside it (a `game-*.bat` option) and put it on the hand-test
  list. These runs are mostly cutscenes; keep each one short and chained by presets.
- **Saves:** before building a run on a save's items, check the preset's actual inventory, C
  buttons and songs. In tests, place Link inside the room you loaded and wait out an entrance
  cutscene first.
- **Player and actors:** Player can't spawn during his update (`PlayRequest`); Player-side
  writes from actors go through `PlayerIface`. Collider spheres updated in draw go in
  `draw_update`. Shadows come from `ActorShape_Init`.
- **Rand:** tests must not depend on Rand luck. A newly ported actor's Rand calls move other
  cases' sequences.
- **Bakes:** textures and meshes are baked at import. A bake change needs a re-import; a change to
  the set of records needs `FORMAT_VERSION` bumped.
- **Goldens:**
  - Prove every changed hash with a build where the cause is taken out (`target/game26proof`, a
    run-time env switch). A trace that changes from a draw-only change is a bug.
  - Record all cases (`--only` rewrites `renders.sha256` with only those), check twice, save the
    new base (`out/golden_base42`), and describe the changes in `golden/README.md`.
  - Don't rebuild `target/game26` while a golden check runs on its binaries; use
    `target/game26proof` for tests meanwhile.
  - `cargo test` doesn't rebuild `oot_sandbox`: rebuild it before a sandbox run.
- **Files:** Bash heredocs choke on apostrophes and eat backslashes, so write Python scripts with
  the Write tool. **Never `sed -i` any file here**; in Python read and write bytes and keep each
  file's line endings.
- **Ids:** check every id against its header. This ROM uses the `_pal` XMLs where they differ
  (`baseroms/gc-eu-mq-dbg/config.yml`). `OOT_VERSION` takes the `>= PAL_1_0` and `>= NTSC_1_1`
  sides, and it isn't `PLATFORM_N64`.
- **Windows:** running `oot` or `oot_sandbox` without `--screenshot`, `--trace` or `--wav` opens a
  window. Ask before opening any window.

**How we work**
- Port function by function from `D:/OOT Modding/oot-main` (52a510f) with the decomp's names.
  Cite constants, mark the game's own bugs "@bug (game)", and order arrays by linkAge, adult
  first.
- Each new actor gets:
  - a schema (ADR 0066) and texts (ADR 0079), with every placement decoding (a param read by
    nothing is a `Kind::Unused` field);
  - its Params and Flags paragraphs;
  - registration in `lib.rs`'s four places;
  - a name in the editor's `actors.rs`;
  - a character in `ootx`'s `characters.rs` if it brings a new skeleton.
- Docs: GAME-11's milestone section; ROADMAP, COVERAGE (`ootx census`, then
  `python scripts/coverage.py`), BACKLOG and `HAND-TESTS-OWED.md` kept in agreement. An ADR for
  the milestone's decisions (0089).
- My view of the real game wins over a decomp-faithful visual. If I report something the C or the
  ROM's data does too, show me and ask.
- The milestone is done by ROADMAP's "How a phase starts and how a milestone is done". End with
  the check commands and a concrete hand-test list (keys and places).
- Commit and push only when I ask.

**Environment:** `CARGO_TARGET_DIR=target/game26`; `OOT_DATA_DIR=out/data27` (absolute for cargo
test); `OOT_SAVE_DIR=out/saves`. Background `&` and `sleep` loops in the foreground are blocked:
use `run_in_background` (a background `until` loop can wait on a file).
