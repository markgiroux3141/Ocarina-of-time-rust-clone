Next piece of work: **GAME-10 milestone 8, the rest of Jabu-Jabu's Belly.**
Read `docs/GAME-10-zoras-waters-and-jabu-jabu.md` first: its survey's "Jabu-Jabu's Belly, Master
Quest" (Ruto in room 3 and room 6, `Bg_Bdan_Objects`' octo platform, the enemies off the way), the
proposal's row 8, and milestones 5's and 7's sections (match their parts when you write milestone
8's). Then ADR 0083 (Ruto whole, the way through Jabu-Jabu) and ADR 0085.
Build it, then give me the hand-test list and `game-*.bat` starts, and stop for my hand test.

**Where things stand**
- Phase 11 (GAME-10): milestones 1 to 7 are done, committed and pushed (last: "GAME-10: the Ocarina
  of Time and the Song of Time"). **Milestones 5's, 6's and 7's hand tests are owed** (I deferred
  them: `docs/HAND-TESTS-OWED.md`); don't wait for them. If I defer a hand test again, add its row
  there.
- Milestone 7 ported `Item_Ocarina` and `En_Ganon_Mant` whole; its runs `Route::Escape` and
  `Route::OcarinaOfTime` reach the presets `zelda-fled` and **`ocarina-of-time`** (Phase 11's exit:
  Phase 12's start). Milestones 8 and 9 are the phase's breadth: their runs start from Jabu-Jabu's
  presets and debug starts, not from the story's end.
- COVERAGE: Phase 11 "in progress", 30 of its 35 ported (`En_Tp`, `En_Vali` this milestone; `En_Fr`,
  `En_Kakasi`, `En_Kakasi3` milestone 9); 214 ported overall.
- 1,699 tests pass, 1 ignored. Goldens: 322 hashes, 241 cases, `out/golden_base36`. Pack format 30
  in `out/data27`; build into `target/game26`.
- Next numbers: ADR 0086, BACKLOG #107, menu entry 166.
- Presets are a chain: each run's end is checked against the next start's preset (`ends_as_preset`,
  less a debug start's room clears and anything the hand test's start adds, as an `extra`).

**Milestone 8's scope** (the proposal's row 8):
- **`En_Bigokuta`** whole (Big Octo, `ovl_En_Bigokuta`, about 921 lines): his arrival on room 6's
  octo platform, the fight's states, his death. The fight itself is the hand test's.
- **`En_Vali`** whole (the Bari, room 6, about 768) and **`En_Tp`** whole (Tailpasaran, room 12,
  about 729).
- **One-points 3070, 3080, 3090** (the platform's rise with Ruto, its descent to Big Octo, its lift
  after him; about 190, logged today by `Bg_Bdan_Objects`). `Bg_Bdan_Objects` type 0's Big Octo
  placeholder becomes the real spawn.
- **Rooms 3, 4, 6, 10, 12, 13 whole:** whatever they place that isn't ported yet (check with `ootx
  scene-info --scene bdan` per room, every layer; the survey lists the known ones).
- **Ruto's room-6 scripts with the real platform:** carried there, `gRutoFoundSapphireCs`, thrown onto
  the platform, `gRutoObtainingSapphireCs` (`INFTABLE_RUTO_HAS_SAPPHIRE`), Big Octo takes her
  (`INFTABLE_RUTO_ABDUCTED`). BACKLOG #104 (child Ruto's 71 jump as her walk to the Sapphire starts):
  check it against the C here, fix or close it.
- **The runs, from debug starts, no fights:**
  - Ruto carried onto room 3's blue switch (0x3C: door 12 opens). `game-jabu.bat carry` is room 3
    by Ruto on `jabu-room-7` (the start to build on).
  - Ruto carried to room 6 (behind the green barrier: the green tentacle's flag 0x0C, a debug
    start's) and thrown onto the platform: her two scripts, the platform, Big Octo's arrival. Its
    end a preset if anything later starts from it.
- `game-*.bat` starts for my hand test: room 3 with Ruto, room 6 with Ruto (the platform), Big
  Octo's fight (beside him, his arrival done), the Bari in room 6, Tailpasaran in room 12, rooms 4,
  10 and 13 if they hold anything new.
- Tests that drive each new actor's states directly (`tests/en_okuta.rs` and
  `tests/ruto_warp.rs` are the models). No scripted fight.

**What milestone 7 found (still applies)**
- A trigger that halts Link short of a waypoint (the drawbridge's) leaves a `Walk` unfinished:
  use `Task::WalkUntil(points, cond, step)`. A dive for something on the bottom: `Task::DiveFor`
  (the dive held until the condition), not `DiveUnder` (it lets go once over the point).
- After a tick, `cs_ctx.frames` is one past the frame the actors saw (the script counts on after
  the actors' update). An actor spawned during another's update first updates on the next frame.
- `Actor_UpdatePos` moves by the velocity times 1.5 (`R_UPDATE_RATE * 0.5`).
- The schemas test wants the whole params read through the schema's field (`TYPE.get(params)`),
  and the profile's object not listed again in `objects`.
- A run's start inside a trigger (`ENTR_HYRULE_FIELD_0`'s spawn is in the drawbridge's) starts
  things at once: check the spawn against the triggers before choosing the start.

**What milestone 6 found (still applies)**
- An actor that another's init spawns (`Actor_Spawn` in `Init`) is in the arena only after the next
  update: idle a frame or two before looking for it.
- A run that ends in a boss's room must end as the room is entered (an intro takes Link at once).
- A run whose last scene puts Link in water ends on "the scene's layer, the transition off, Link
  free", not on a standing settle.
- Check a survey's "nearest"/"first" claims against the C's loop before writing them into a test.

**Helpers (agents)**
- **At most two helpers at once.** A fresh helper with a tight brief costs about 250k to 600k tokens
  (milestone 7's `En_Ganon_Mant`: 256k). Don't resume a stopped one; start a fresh one.
- Suggested split: one helper for `En_Bigokuta` (the largest; with one-points 3070, 3080, 3090 if
  they're tied to him), one for `En_Vali` and `En_Tp`. Keep `Bg_Bdan_Objects`' hookup, Ruto's room-6
  check, the runs, the presets and the docs in the main session.
- Helper rules: `out/handoff_m7/HELPER_RULES.md` (copy to `out/handoff_m8/`, update paths and
  numbers): their own worktree under `.claude/worktrees/` on a branch from HEAD, their own pack
  folder (`out/data27_<name>`, a copy of `out/data27`), their own target; only their own tests,
  `--test schemas` and `cargo test -p layering`; no subagents; commit in their worktree and report
  briefly.
- Merge with `git diff <base>..<branch> > patch` and `git apply --reject`, then the `.rej` files by
  hand. The usual conflict is `oot_actors/src/lib.rs`'s PROFILES line (one long line). **`git apply`
  writes every file it touches with CRLF:** afterwards put back LF on every file that isn't CRLF
  (only `oot_actors/src/lib.rs`, `oot_game/src/effect/mod.rs`, the editor's `actors.rs`, `ootx`'s
  `characters.rs`, `oot_game/src/save.rs`, `oot_game/src/item.rs` and `docs/BACKLOG.md` are CRLF
  in the working tree; check with Python). Remove the worktree, its branch and its pack copy after
  the merge.
- Long local jobs (the test suite, golden checks, imports) cost no tokens: run them as background
  shells. Don't run `oot_sandbox` for shots while a background build may relink it (the build
  fails to replace the exe).

**Lessons (all still apply)**
- Never script an enemy fight or the long story chain, even if a brief names one. Give a debug start
  beside the enemy (a `game-*.bat` option) and put the fight on the hand-test list.
- Settle reachability only as far as you need to reach and exercise the actors. If it gets deep, note
  it in a line, give a debug start past it, and move on. Coverage matters more than completability.
- Check a spawn's or start's surroundings before starting a run from it: probe the floor
  (`col.entity_raycast_down`) and the water (`col.water_surface`). Probes kept in `out/handoff_m6/
  probe_jabu.rs` (Jabu-Jabu's rooms) and `out/handoff_m7/probe_moat.rs` (copy one into
  `oot_actors/tests` to run it with `--ignored`, and remove it after).
- A `WaitFor(Cond::Switch(..))` can finish before the cutscene the switch starts: follow it with
  `Task::WaitCameras`. Use `Task::Hurry` for run-off jumps; `Task::Texts(cond, step)` presses A
  through texts until any condition holds (also the way to wait on a cutscene's end state).
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
  (`target/game26/release/ootx.exe import` with `OOT_DATA_DIR` set).
- Goldens: prove every changed hash with a build where the cause is taken out
  (`target/game26proof`, run-time env switches; `out/handoff_m6/proof_m6.py` applies and restores
  them, checking its backups first: adapt its targets; **a proof script must never write a file
  before it has its backup**). A trace that changes from a draw-only change is a bug. Record all
  cases (`record --only` rewrites the list), check twice, save the new base (`out/golden_base37`),
  and describe changes in `golden/README.md`. Don't rebuild `target/game26` while a golden check
  runs on its binaries, and don't run the workspace tests while editing what they compile.
- Look at new golden shots headlessly before recording them (`oot_sandbox ... --frames N
  --screenshot out/...png`, then read the PNG).
- Bash heredocs choke on apostrophes and eat backslashes (also inside Python strings in a heredoc:
  `out\run` became a carriage return in milestone 7): write longer Python scripts with the Write
  tool, and build backslashes with `chr(92)`. **Never `sed -i` any file here**; in Python read and
  write bytes and keep each file's line endings. `rm` with a glob gets blocked: remove explicit
  paths.
- Check every id against its header. This ROM uses the `_pal` XMLs where they differ
  (`baseroms/gc-eu-mq-dbg/config.yml`).
- Running `oot` or `oot_sandbox` without `--screenshot` or `--trace` opens a window. Ask before
  opening any window. Headless screenshots: `oot_sandbox --entrance ... --script still --frames N
  --screenshot out/...png`.

**How we work**
- Port function by function from `D:/OOT Modding/oot-main` (52a510f) with the decomp's names. Cite
  constants, mark the game's own bugs "@bug (game)", and order arrays by linkAge, adult first.
- Each actor: a schema (ADR 0066), texts (ADR 0079), Params and Flags paragraphs in its module docs,
  registered in `lib.rs`'s four places, named in the editor's `actors.rs`, a character in `ootx`'s
  `characters.rs` for a new skeleton.
- Docs: GAME-10's milestone 8 section, an ADR (0086) for the milestone's decisions; ROADMAP, COVERAGE
  (`ootx census` then `python scripts/coverage.py`), BACKLOG and `HAND-TESTS-OWED.md` kept in
  agreement.
- My view of the real game wins over a decomp-faithful visual. If I report something the C or the
  ROM's data does too, show me and ask.
- Commit and push only when I ask.

**Environment:** `CARGO_TARGET_DIR=target/game26`; `OOT_DATA_DIR=out/data27` (absolute for cargo
test); `OOT_SAVE_DIR=out/saves`. Background `&` and `sleep` loops in the foreground are blocked: use
`run_in_background` (a background `until` loop can wait on a file).
