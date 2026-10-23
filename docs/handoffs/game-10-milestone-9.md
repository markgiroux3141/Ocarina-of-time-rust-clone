Next piece of work: **GAME-10 milestone 9, the frogs and the scarecrows** (the last of Phase 11).
Read `docs/GAME-10-zoras-waters-and-jabu-jabu.md` first: its survey's route section (the frogs on
Zora's River's log, the lake's scarecrows), the systems table's "frogs' and scarecrows' ocarina" row,
the proposal's row 9, the user's call 3, and milestone 8's section (match its parts when you write
milestone 9's). Then BACKLOG #47 and ADR 0061 (the ocarina's scope), ADR 0086.
Build it, then give me the hand-test list and `game-*.bat` starts, and stop for my hand test.

**Where things stand**
- Phase 11 (GAME-10): milestones 1 to 8 are done, committed and pushed (last: "GAME-10: the rest of
  Jabu-Jabu's Belly", c686b38), **all hand-tested** (5 to 8 on 2026-10-22/23; `HAND-TESTS-OWED.md`
  has no open rows). If I defer a hand test, add its row there.
- COVERAGE: Phase 11 "in progress", 32 of its 35 ported; left: `En_Fr`, `En_Kakasi`, `En_Kakasi3`.
  `En_Ms` (the bean seller) is already ported (milestone 2). 217 ported overall.
- 1,737 tests pass, 1 ignored. Goldens: 331 hashes, 247 cases, `out/golden_base39`. **Pack format 31**
  in `out/data27` (bumped for the renderer's static `LOD_FRACTION`); build into `target/game26`.
- Next numbers: ADR 0087, BACKLOG #110, menu entry 166.
- Presets are a chain: each run's end is checked against the next start's preset (`ends_as_preset`,
  less a debug start's room clears and anything the start adds, as an `extra`). Phase 11's exit preset
  is `ocarina-of-time` (Phase 12's start); milestone 9's runs start from debug starts on it or earlier.

**Milestone 9's scope** (the proposal's row 9, the user's call 3: here, not Phase 20):
- **`En_Fr`** whole (`ovl_En_Fr`, 1,118 lines; `object_fr`): Zora's River (`spot03`) room 0's log,
  layers 0 and 1: params 0 the invisible song spot at (990, 205, -1220), 1 to 5 the frogs. The five
  songs (Zelda's Lullaby, Epona's, Saria's, the Sun's Song, the Song of Time) give a purple rupee the
  first time each (`EVENTCHKINF_SONGS_FOR_FROGS_*` 0xD1 to 0xD5), the Song of Storms a piece of heart
  (0xD6), and with all six their song game (0x40AB, `OCARINA_ACTION_FROGS`, a piece of heart, 0xD0).
  Check the survey's flags and texts against the C before using them.
- **One-point 4110** (the frogs' camera, about 45 lines).
- **The frogs' ocarina modes** (`MSGMODE_FROGS_*`, `z_message.c:3882-3903`; the mode's start is
  ported) and **the song recording** (`AudioOcarina_SetRecordingSong`, `_SetRecordingState`,
  `_RecordSong`, `_PlayLongScarecrowSong`, `general.c:1928-2187`; the box's `MSGMODE_SCARECROW_*`):
  BACKLOG #47, now in scope.
- **`En_Kakasi`** (365) and **`En_Kakasi3`** (457, Bonooru: records a song), Lake Hylia (`spot06`)
  layers 0 and 1, `object_ka`. The adult layers' `En_Kakasi2` (Pierre, three placed in layers 2 and
  3) is adult content: check what the C does with it here and decide (a placeholder chosen at init,
  as the phase's adult parts, unless it's small).
- **The runs, from debug starts, no fights:** the songs played on the frogs' log (each frog's rupee;
  the song game if the save can reach it), and Bonooru recording a song. The `ocarina-of-time`
  preset has the Ocarina of Time and five songs but **no ocarina on a C button** and no Song of
  Storms: put the ocarina on a C button from the item page in the run (`Task::OpenMenu`,
  `MenuCursor`, `EquipOnC`), or make a debug preset; the Song of Storms (adult, the windmill) is a
  debug start's if the frogs' heart and game are run at all.
- `game-*.bat` starts for my hand test: the frogs' log, the lake's scarecrows; a new
  `game-frogs.bat` or options on an existing one (menu 166 on).
- Tests that drive each new actor's states directly (`tests/en_okuta.rs` and `tests/ruto_warp.rs`
  are the models), and the recording and frog modes against the C.
- Then **Phase 11's exit review** (as GAME-09's last section): what's left and where, Phase 12's start.

**What milestone 8 found (still applies)**
- **Settle a way with the real game's walkthroughs when the geometry is unclear** (Wikibooks' MQ
  pages fetch; StrategyWiki and ZeldaDungeon refuse): room 3's way up was the spout, not a slope.
  Probe the floors (`out/handoff_m8/probe_m8.rs`), then drive Link with draft waypoints and
  `JABU_TRACE`-style traces before writing the run.
- `Task::LiftWho(Who, Step)` lifts an actor by `Who` (`Who::Ruto`); `Task::PutDown` presses A again
  if a switch's camera took the first. A carried actor put down lands about 6 in front of Link.
- **Diagnostic log lines** (silent unless `RUST_LOG` is set): `[target]` lock on/off, `[camera]`
  mode changes, `[frame]` slow display frames at info; `[sfx] start` at debug; `OOT_PAD_DEBUG=1`
  logs the pad's raw events. PowerShell's `2>` writes the log as **UTF-16**: decode it before
  grepping. A log from my run found BACKLOG #108 where headless reproductions failed.
- **Player has two `Player_UpdateHostileLockOn`s:** `player_update_hostile_lock_on_env(env)` is the
  C's function; `update_hostile_lock_on()` is only its clearing half. Never call the half alone.
- **The renderer's `LOD_FRACTION`** is only the static case (`Material::lod_frac`: 0xFF with sharpen
  and detail off and `gSPTexture` level 0); per-pixel is BACKLOG #109. TMEM loads wrap round 4 KB.
- BACKLOG #107: room 4's floating Flobbery voids Link out, as the C does (kept, my call).

**What milestones 6 and 7 found (still apply)**
- A trigger that halts Link short of a waypoint leaves a `Walk` unfinished: `Task::WalkUntil(points,
  cond, step)`. A dive for something on the bottom: `Task::DiveFor`.
- After a tick, `cs_ctx.frames` is one past the frame the actors saw. An actor spawned during
  another's update first updates on the next frame; one an init spawns is in the arena only after
  the next update.
- `Actor_UpdatePos` moves by the velocity times 1.5 (`R_UPDATE_RATE * 0.5`): a grounded Link's y dips
  7.5 under the floor before the floor check.
- The schemas test wants the whole params read through the schema's field (`TYPE.get(params)`), and
  the profile's object not listed again in `objects`.
- A run's start inside a trigger starts things at once: check the spawn against the triggers.
- Check a survey's "nearest"/"first" claims against the C's loop before writing them into a test.

**Helpers (agents)**
- **At most two helpers at once.** A fresh helper with a tight brief costs about 250k to 600k tokens
  (milestone 8's `En_Bigokuta` 344k, `En_Vali` + `En_Tp` 402k). Don't resume a stopped one.
- Suggested split: one helper for `En_Fr` (with one-point 4110 if it's tied to it), one for
  `En_Kakasi` and `En_Kakasi3`. Keep the ocarina's frog modes and recording (framework: the message
  box and the ocarina), the runs, the presets and the docs in the main session, or brief a helper on
  them only once the actors' needs are clear.
- Helper rules: `out/handoff_m8/HELPER_RULES.md` (copy to `out/handoff_m9/`, update the paths, the
  pack format and numbers): their own worktree under `.claude/worktrees/` on a branch from HEAD, their
  own pack folder (`out/data27_<name>`, a copy of `out/data27`), their own target; only their own
  tests, `--test schemas` and `cargo test -p layering`; no subagents; commit in their worktree and
  report briefly.
- Merge with `git diff <base>..<branch> > patch` and `git apply --reject`, then the `.rej` files by
  hand (the usual one is `oot_actors/src/lib.rs`'s PROFILES line). **`git apply` writes every file it
  touches with CRLF:** afterwards put each file back to its line endings at HEAD (check with Python:
  `git show HEAD:<path>` for the original; many are CRLF in the working tree, among them `player.rs`,
  `door_warp1.rs`, `oot_actors/src/lib.rs`, `save.rs`, `item.rs`, `effect/mod.rs`, the editor's
  `actors.rs`, `ootx`'s `characters.rs`, `docs/BACKLOG.md`, the `.bat` files; new files are LF).
  Remove the worktree, its branch and its pack copy after the merge.
- Long local jobs (the test suite, golden checks, imports) cost no tokens: run them as background
  shells. Don't run `oot_sandbox` for shots while a background build may relink it.

**Lessons (all still apply)**
- Never script an enemy fight or the long story chain, even if a brief names one. Give a debug start
  beside the enemy (a `game-*.bat` option) and put the fight on the hand-test list.
- Settle reachability only as far as you need to reach and exercise the actors. If it gets deep, note
  it in a line, give a debug start past it, and move on. Coverage matters more than completability.
- Check a spawn's or start's surroundings before starting a run from it: probe the floor
  (`col.entity_raycast_down`) and the water (`col.water_surface`). Probes kept in `out/handoff_m6/
  probe_jabu.rs`, `out/handoff_m7/probe_moat.rs` and `out/handoff_m8/probe_m8.rs` (copy one into
  `oot_actors/tests` to run it with `--ignored`, and remove it after). Test file paths are relative to
  the crate: give probes an absolute output path.
- A `WaitFor(Cond::Switch(..))` can finish before the cutscene the switch starts: follow it with
  `Task::WaitCameras`. Use `Task::Hurry` for run-off jumps; `Task::Texts(cond, step)` presses A
  through texts until any condition holds (also the way to wait on a state, e.g. a spout's lift).
- Before building a run on a save's items, check the preset's actual inventory, C buttons and songs.
- In tests, place Link inside the room you loaded, and wait out an entrance cutscene before testing
  anything Player does on his own.
- Player can't spawn during his update: use a `PlayRequest`. Player-side writes from actors go
  through `PlayerIface`.
- Collider spheres updated in draw go in `draw_update`. Shadows come from `ActorShape_Init`. Anything
  drawn from collision must use the captured game frame, not the latest collision at 60 Hz.
- A read of another actor's params needs a `// schema: another actor's params read whole` note on
  the line before. A schema's `TextLine::when` names the field's name.
- Tests must not depend on Rand luck. A newly ported actor's Rand calls move other cases' sequences.
- Textures and meshes are baked at import: a bake change needs a re-import
  (`target/game26/release/ootx.exe import` with `OOT_DATA_DIR` set); a change to what a bake stores
  (`Material`, a draw list's fields) needs `FORMAT_VERSION` bumped.
- Goldens: prove every changed hash with a build where the cause is taken out
  (`target/game26proof`, a run-time env switch; `out/handoff_m8/proof_lod.sh` is the latest example:
  it keeps a backup before editing and restores it). A trace that changes from a draw-only change is a
  bug. Record all cases (`record --only` rewrites the list), check twice, save the new base
  (`out/golden_base40`), and describe changes in `golden/README.md`. Don't rebuild `target/game26`
  while a golden check runs on its binaries, and don't run the workspace tests while editing what
  they compile.
- Look at new golden shots headlessly before recording them (`oot_sandbox ... --frames N
  --screenshot out/...png`, then read the PNG): an actor that only appears when Link is near needs
  Link near it and the title card gone (about 150 frames after entering).
- Bash heredocs choke on apostrophes and eat backslashes: write longer Python scripts with the Write
  tool, and build backslashes with `chr(92)` or raw strings. **Never `sed -i` any file here**; in
  Python read and write bytes and keep each file's line endings. `rm` with a glob gets blocked: remove
  explicit paths.
- Check every id against its header. This ROM uses the `_pal` XMLs where they differ
  (`baseroms/gc-eu-mq-dbg/config.yml`).
- Running `oot` or `oot_sandbox` without `--screenshot`, `--trace` or `--wav` opens a window. Ask
  before opening any window. Headless: `oot_sandbox --entrance ... --script still --frames N
  --screenshot out/...png`; `--audio-log` lists every sound request by frame, `--wav` writes the sound.

**How we work**
- Port function by function from `D:/OOT Modding/oot-main` (52a510f) with the decomp's names. Cite
  constants, mark the game's own bugs "@bug (game)", and order arrays by linkAge, adult first.
- Each actor: a schema (ADR 0066), texts (ADR 0079), Params and Flags paragraphs in its module docs,
  registered in `lib.rs`'s four places, named in the editor's `actors.rs`, a character in `ootx`'s
  `characters.rs` for a new skeleton.
- Docs: GAME-10's milestone 9 section, an ADR (0087) for the milestone's decisions; ROADMAP, COVERAGE
  (`ootx census` then `python scripts/coverage.py`), BACKLOG and `HAND-TESTS-OWED.md` kept in
  agreement; then Phase 11's exit review.
- My view of the real game wins over a decomp-faithful visual. If I report something the C or the
  ROM's data does too, show me and ask.
- Commit and push only when I ask.

**Environment:** `CARGO_TARGET_DIR=target/game26`; `OOT_DATA_DIR=out/data27` (absolute for cargo
test); `OOT_SAVE_DIR=out/saves`. Background `&` and `sleep` loops in the foreground are blocked: use
`run_in_background` (a background `until` loop can wait on a file).
