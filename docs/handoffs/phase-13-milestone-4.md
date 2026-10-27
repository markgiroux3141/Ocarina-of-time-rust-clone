# Handoff: GAME-12 milestone 4, the ranch after Epona and the field's Poes

Written 2026-10-26, at the end of the session that built GAME-12 milestones 1 to 3. It's for the
next session, which builds milestone 4: Phase 13's last.

## Where things stand

- **Phase 13** (GAME-12, `docs/GAME-12-epona-and-the-adult-field.md`) has its exit met. Done,
  hand-tested and pushed:
  - milestone 1, riding Epona: `35e1320`, ADR 0091;
  - milestone 2, Ingo, the rental and the races: `5699b62`, ADR 0092;
  - milestone 3, Epona won and out: `eddf199`, ADR 0093.
- The preset **`epona-won`** is Phase 14's start.
- **Milestone 4 is what's left of the phase.** Read GAME-12's "The proposal" (its row 4) and "The
  user's calls", then the three milestone sections for the model of what a milestone writes up.
  The user's calls that bear on it:
  - `En_Po_Field` in this phase, whole, its fight the hand test's;
  - Malon's course: its start in a run, the laps the hand test's.
- **Next numbers:** ADR 0094, BACKLOG #112, run-script menu 187.
- Tests: 1,839 pass, 1 ignored. Goldens: 390 hashes, 286 cases (`out/golden_base46`). Pack format
  35 in `out/data27`.

## The milestone

The proposal's row 4, as corrected by what's been built since:
- **`En_Ma3`** whole (`ovl_En_Ma3/z_en_ma3.c`, 444 lines): adult Malon by the corral after Epona,
  her obstacle course (the offer, the start, the high score, the cow's unlock).
  - The course's referee is already ported whole: `En_Horse_Game_Check` type 4,
    `crates/game/oot_actors/src/en_horse_game_check.rs`, with two `@bug (game)`s noted.
  - The course's fences are `Bg_Umajump` type 1 on the ranch, killed until Epona is had.
- **`En_Ma2`'s adult** (`ovl_En_Ma2/z_en_ma2.c`, 428 lines): the child's half is ported
  (`crates/game/oot_actors/src/en_ma2.rs`). For the adult's spawn indexes her init spawns a
  placeholder: see its "Not ported" docs and the `indirect("not ported (the adult's)")` flags in
  its schema. That covers her singing, Epona's Song played to her, and her texts and flags.
- **`En_Po_Field`** whole (`ovl_En_Po_Field/z_en_po_field.c`, 1,018 lines): Hyrule Field's Poes
  and Big Poes, which appear to Link riding at night. The Big Poe's soul goes in a bottle (it ties
  into Phase 20's Poe collector).
- **Already done, don't redo:** Ingo's obedient and sleeping roles came whole with milestone 2's
  `En_In`.

**Runs** (short debug starts, no fights: the user's standing rule):
- (a) Malon by day on `epona-won`: her texts and the course's start; the laps are the hand test's.
- (b) Malon by night before Epona (`adult-awake` at night, or a preset): Epona's Song played to her.
- (c) The field on Epona near a Big Poe's place: it appears. The fight and the bottle are the hand
  test's.

**Helpers:** at most two at once, no sub-agents. The proposal's split:
- one helper on `En_Po_Field`;
- one on `En_Ma3` with `En_Ma2`'s adult;
- the main session keeps the runs, presets, goldens and docs.

The last helper brief (in this session's history, not saved) worked well. It gave the helper its
own `CARGO_TARGET_DIR` (`target/game26h`), the files it mustn't edit (the ones the main session is
in), the "never `sed -i`" rule and the report format. Tell the helpers which `NA_SE_*` and
`NA_BGM_*` constants the main session adds, so the two sides don't both add them.

## Before you start: the survey

Before the build, check (and record what you find in GAME-12):
- **Where the actors are placed:** `ootx scene-info --scene spot20 --layer N` for the ranch,
  `spot00` for the field (layers 0 and 1 child, 2 and 3 adult, 4 and up cutscenes), and the Lon
  Lon buildings.
- **When a Big Poe shows up.** The C decides: riding, at night, near its place.
- **Whether Player's bottle catch already handles a Poe's soul.** `Task::CatchInBottle` exists for
  bugs and fish.
- **Malon's course:** its start and record (the high score is in the save).

## Standing rules

From the user, in memory and the earlier briefs:
- **Commit and push only when the user asks.** Stage only this work's files. Another session works
  on the level editor in the same tree (`crates/tools/overworld*`, `docs/EDITOR-*.md`, sometimes a
  row in `docs/ROADMAP.md`'s editor table). If a shared file holds both, stage it by hunk:
  `git diff f > p`, cut the patch to our hunks, `git apply --cached p`. Say whether the separation
  was clean.
- **File edits:** never `sed -i` any file. In Python, read and write bytes and keep each file's
  line endings. Some files are CRLF in the working tree: `save.rs`, `player.rs`, `lib.rs`,
  `actor_ctx.rs`, `actors.rs`, `menu.bat`, the `.bat`s. Others are LF.
- **Write scripts with the Write tool.** Bash heredocs here mangle backslashes (`out\run` became a
  CR once) and sometimes quotes.
- **Never script enemy fights or long routes;** give debug starts and a hand-test list.
- **Ask before opening any window.** `oot`/`oot_sandbox` without `--screenshot`, `--trace` or
  `--wav` opens one. For an entrance-only sandbox shot use `--script still`.
- **Port function by function** with the decomp's names. Cite each constant with its header and
  check every id against it. Mark game bugs `@bug (game)`.
- **Each new actor needs:**
  - a schema with Params and Flags paragraphs in its module docs (ADR 0066, 0062);
  - its texts;
  - registration in `crates/game/oot_actors/src/lib.rs`'s four places;
  - an editor name in `crates/tools/overworld_editor/src/actors.rs`;
  - an `ootx` character for a new skeleton.
- **`tests/schemas.rs` must pass:**
  - Params and rotations are read only through the schema's `Field` consts.
  - Flag calls match the schema; a computed flag or text id gets a `// schema:` note.
  - Every text id literal is in `texts` or noted.
  - A placement's value must be in a `Kind::Named` list: e.g. `-1` for `0xFFFF` on a whole field.
- **Goldens:**
  - Check before recording.
  - Prove every changed hash with a build where the cause is taken out: `target/game26proof`.
    Patch the cause out, build, `--only` the cases, restore.
  - Record, check again, save `out/golden_base47`, log it in `golden/README.md`.
  - Don't run `cargo` in `target/game26` while a golden check runs.
- **Keep the docs in agreement:**
  - GAME-12's milestone section, with its subsections (the answer, what was built, results,
    corrections, known gaps, how to check, by hand);
  - the status table;
  - ROADMAP's Phase 13 paragraph (and Phase 13 done, if the user calls it);
  - COVERAGE (`ootx census` with a rebuilt `ootx`, then `python scripts/coverage.py`);
  - BACKLOG, the ADR and its index row, the golden log;
  - the run scripts (`game-`, `test-`, `sandbox-` bats, CRLF), the menu and `scripts/run/README.md`;
  - `docs/HAND-TESTS-OWED.md` only if a hand test is deferred past a commit.
- **Environment:** `CARGO_TARGET_DIR=target/game26`; `OOT_DATA_DIR` absolute,
  `D:/Claude Code Projects/OOT Clone/out/data27`; `OOT_SAVE_DIR=out/saves`. Decomp:
  `D:/OOT Modding/oot-main`. A new bake bumps `pack.rs`'s `FORMAT_VERSION` and needs a
  re-import into the data dir.
- **Checks before reporting:**
  - `cargo test --release --workspace`;
  - `cargo test -p layering`;
  - no warnings;
  - the golden check.

## What this phase taught that the next milestone will meet

- **Runs live in `crates/game/oot_actors/src/playthrough.rs`.** A route needs entries in:
  - `entrance`, `preset`, `start` (a debug start), `script`, `max_frames`, `needs_audio` (any
    ocarina), the `from_script` list and `tasks`;
  - steps in `Step` and `Step::name`.

  The tests drive the routes as in `tests/ingo_runs.rs` or `tests/riding_runs.rs`.
- **Riding tasks:** `Mount`, `Ride`, `RideExit`, `RideIn`, `RideUntil`, `RideTalk`, `Dismount`.
  - A scripted gallop turns slowly: the runs steer at 4× the heading error, and stop and turn in
    place for a sharp turn.
  - A spur works from any gait.
  - A high obstacle (over 72) needs speed above 13.8.
- **Player clears `rideActor` at the end of each update.** A script sees a horse's offer by the
  "Climb" do-action. On a standing horse A means "Down", so press A to talk only on Player's
  `target_actor` offer.
- **A route's `start_room` reloads the room's actors:** don't give one where an actor spawned at
  init (Epona) must survive.
- **Entering Lon Lon Ranch at spawn 0 resets the horses' `eventInf` word** (`En_In`'s
  `sIsObedient`). Enter by `_6`, `_7` or `_8` to keep a race state.
- **Text ids:** the decomp has no extracted message text here. Print a message from the pack:
  `pack.messages()`, `entries` and `data`; the old probe printed printable ASCII between one entry's
  segment and the next.
- **Texts from a chained box** (0x2030 runs on into 0x2031) both show in a run's `texts`.
