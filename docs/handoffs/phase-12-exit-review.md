# Handoff: Phase 12's exit review, then Phase 13's survey

Paste the prompt below into a fresh session (from the repo root). Written 2026-10-24, after GAME-11
milestone 3 (547b6a4, ADR 0090), the last of Phase 12's milestones.

---

## Prompt

Next piece of work: **Phase 12's exit review** (GAME-11's last section), then **Phase 13's survey**
(Epona and the adult Hyrule Field). Do the review, show it to me, then go on to the survey and stop
at its proposal for my calls.

**Where things stand**
- Phase 12 (GAME-11, `docs/GAME-11-the-temple-of-time-and-adult-link.md`) has all three milestones
  built, hand-tested, committed and pushed:
  - 1, adult Link and the time machine: fe527aa (handoff 05b32a5), ADR 0088;
  - 2, the Door of Time to the Chamber of the Sages and out, **the exit**: 0048fd3 (handoff
    3a1e878), ADR 0089;
  - 3, Sheik's Prelude, time travel both ways and the warp: 547b6a4, ADR 0090.
- Read GAME-11 whole: the survey, the proposal, the user's calls, and the three milestone sections
  (what was built, results, corrections, known gaps).
- COVERAGE: Phase 12's scenes place 5 actors, all 5 ported; 227 ported overall. `scripts/coverage.py`'s
  `SCENES` table still says "in progress" for phase 12.
- 1,811 tests pass, 1 ignored. Goldens: 369 hashes, 272 cases, `out/golden_base43`. **Pack format
  34** in `out/data27` (`scripts/run/_env.bat`); build into `target/game26`.
- Next numbers: ADR 0091, BACKLOG #111, menu entry 178.
- The story's presets this phase: `door-of-time`, `light-medallion`, **`adult-awake`** (the
  awakening's end, and the way out and the ruins runs start from it). Debug presets, off the story:
  `master-sword`, `master-sword-adult` (milestone 1), `prelude` (with the Forest Temple's
  `EVENTCHKINF_48`), `prelude-learned`, `child-again` (milestone 3).
- `docs/HAND-TESTS-OWED.md` still has GAME-10 milestone 9's row (the frogs and scarecrows) owed.

**The exit review** (model it on GAME-10's "Phase 11's end (the exit review, 2026-10-23)": read that
section first and match its parts):
1. **The exit, met:** adult Link out of the Temple (milestone 2, `Route::TempleExit`, then
   `Route::Ruins` to the field by the broken drawbridge), and time travel both ways (milestones 1
   and 3); the commits.
2. **What the phase built:** the actors (`Bg_Toki_Swd`, `Door_Toki`, `En_Rl`, `Demo_6K` 14 to 19,
   `En_Xc` 9, 1, 2 and the pedestal block, `Bg_Spot00_Break`; ported count at the phase's start, 221,
   against its end), the systems (the age switch and `Inventory_SwapAgeEquipment`, Player's time
   travel and cutscene modes 18 and 30, the adult's draw details and the tunic's dynamic segment,
   `SDC_CHAMBER_OF_THE_SAGES`, `chamberCutsceneNum`, Player's draw switch `ActorImpl::has_draw`,
   the run tasks `Task::Pedestal`, `WalkUntil` in turn, `MenuCursor` by the cursor's rules), the
   runs with their frames and presets, and the numbers (tests: 1,771 at Phase 11's end, 1,811 now;
   goldens 337 and 251 at `out/golden_base40`, now 369 and 272 at `out/golden_base43`; menu entries
   169 to 177; pack formats 33 and 34; ADRs 0088 to 0090).
3. **Phase 13's start: the preset.** The story's save at the phase's end is `adult-awake` (the
   awakening's end; the way out and the ruins add no flags: their tests check that). Confirm it, and
   say what Link has with it, **read from the preset** (a probe or a test), not from this brief:
   the age, B and the C buttons, the items, the equipment, the songs (no Prelude: that's the debug
   `prelude-learned`), the medallions, hearts, magic, rupees. Note where Phase 13's route starts
   (Lon Lon Ranch for an adult: Ingo's and Malon's adult layers) and which entrance a debug start
   would use.
4. **Left for later:** BACKLOG #110 (the Chamber's medallion scripts and `Door_Warp1`'s adult warps:
   Phases 14 to 18), the adult's parts chosen at init that the survey assigned to later phases (its
   table), the equipment page and Link drawn in it (Phase 20, the user's call 3), `En_Xc`'s other
   types (the songs: Phases 14 to 17; the reveal: 19), `Demo_6K`'s others, the debug presets that
   wait for Phase 14 (`prelude`), and the frogs' owed hand test. Group them by where they'll be
   picked up, as Phase 11's review did.
5. **Checked:** ROADMAP (Phase 12 done, the "Where things stand" prose if it names Phase 12),
   COVERAGE (`coverage.py`'s `SCENES` phase 12 set to done; regenerate with `ootx census` then
   `python scripts/coverage.py`), the ADR index (0088 to 0090) and BACKLOG (#26, #37, #57 closed,
   #110 open) all agree with GAME-11. Then `cargo test --release --workspace --no-fail-fast`,
   `cargo test -p layering`, and `python scripts/golden.py check --bin-dir target/game26/release`
   (369 of 369 expected; nothing in the review should change a hash).

**Then Phase 13's survey** (Lon Lon Ranch and Hyrule Field as an adult: ROADMAP's outline row 13,
"Riding Epona (`En_Horse` whole: the horse camera, jumps), Ingo's races", exit "Epona won"; and
COVERAGE's Phase 13 row: `Bg_Ingate`, `Bg_Umajump`, `En_Horse`, `En_Horse_Game_Check`, `En_Ma3`,
`En_Po_Field`, none ported). Follow ROADMAP's "How a phase starts and how a milestone is done":
- The scenes and their adult layers (`ootx scene-info` on `spot20` (the ranch), its buildings
  (`souko`, the stables), `spot00` (the field's layers 2 and 3); the entrance table's adult rows).
- The route an adult takes from `adult-awake`: to the ranch, Ingo, Malon and Epona's Song, the
  races and the bet, Epona won, the jump out (the gate shut), and riding in the field.
- What's already ported and reused: `En_Horse_Normal`, `En_Horse_Link_Child` (the child's horse),
  `En_In` and `En_Ma2` with their adult parts left as placeholders chosen at init (`en_in.rs`,
  `en_ma2.rs`), `En_Ma1`, `En_Ta`, the skin system for the horses (GAME-04b), Epona's Song and its
  ocarina handling.
- The systems needed: Player on a horse (mounting, riding, the actions and the camera:
  `CAM_SET_HORSE`, `Camera_Normal3`?), `En_Horse` (the largest overlay here: check its size), the
  race's game checks, the fences' jumps, the Poes in the field (`En_Po_Field`) and the bow if they
  need it (a fight: the hand test's, not a run). Check against the C what an adult on Epona needs
  that the spikes' Player lacks.
- The sizes in lines of C.
- Things to check for me.

Then a **proposal**: milestones in the story's order with the exit early if it can be, a run for
each (short debug starts, chained by presets, no fights, no horse race that's a long scripted chase
unless it's short and deterministic: say if it isn't), the helpers' share (at most two at once),
and the calls that are mine. Write it as a new `docs/GAME-12-<name>.md`. Stop there for my calls.

**Lessons (all still apply)**
- **Fights and long routes:** never script an enemy fight or the long story chain, even if a brief
  names one. Give a debug start beside it (a `game-*.bat` option) and put it on the hand-test list.
  Coverage matters more than completability: if reachability gets deep, note it and give a debug
  start past it.
- **Debug starts and the story's flags:**
  - Enter the way the story does, then move with `--room`/`--at` (`Route::start_room`, `start`): an
    entrance can draw what the story never shows there (`Demo_Effect`'s ring at
    `ENTR_TEMPLE_OF_TIME_4`).
  - A preset must hold every flag the story has by that point, or placed actors act on the gap.
    End a run before the next scene's `Play_Init` if an actor there sets a flag at once
    (`light-medallion` ends as terminator 18's transition starts: `En_Xc` 9 sets `EVENTCHKINF_C5` on
    init).
  - The sandbox needs `--child` for a route whose preset makes Link an adult (it has no `--adult`).
- **Saves:** before building a run on a save's items, read the preset's actual inventory, buttons
  and songs. `Inventory_SwapAgeEquipment` never clears what it stored: after a round trip
  `adultEquips` holds the adult's buttons and `childEquips` keeps whatever sword nibble the child
  wore (the C's).
- **Song spots** take the ocarina only within 50 (where they set `PLAYER_STATE2_23`): put Link
  right at them, or Player plays freely. An ocarina route's test needs the audio side.
- **Run tasks:** `Task::Walk` and `Task::WalkUntil` take their points in turn; `Task::MenuCursor`
  follows the item page's moves (empty slots skipped, rows wrapped). Probe floors and walls before a
  walk (`out/handoff_m9/probe_m9.rs`: copy it into `oot_actors/tests`, run with `--ignored` and
  `PROBE_OUT` absolute, remove it after); lines along both axes find round obstacles (the ruined
  market's fountain).
- **Player and actors:** Player can't spawn during his update (`PlayRequest`); Player-side writes
  from actors go through `PlayerIface`, except a plain write to his actor's rotation the C makes
  (`En_Rl`'s, ADR 0089). `actor.draw` NULL is `ActorImpl::has_draw` (ADR 0090). Collider spheres
  updated in draw go in `draw_update`. Shadows come from `ActorShape_Init`. A schema must name every
  rotation a placement sets (`Bg_Spot00_Break`'s leaning fences failed `placements_decode`).
  `En_Arrow`'s init turns a cutscene nut into `ARROW_NUT`.
- **Rand:** tests must not depend on Rand luck. A newly ported actor's Rand calls move other cases'
  sequences.
- **Bakes:** textures and meshes are baked at import. A bake change needs a re-import; a change to
  the set of records needs `FORMAT_VERSION` bumped (re-import into `out/data27`).
- **Goldens:** prove every changed hash with a build where the cause is taken out
  (`target/game26proof`, a run-time env switch); a trace that changes from a draw-only change is a
  bug. Check all cases before recording, record all cases (`--only` rewrites `renders.sha256` with
  only those), check twice, save the new base (`out/golden_base44`), and describe the changes in
  `golden/README.md`. Don't rebuild `target/game26` while a golden check runs on its binaries; run
  the workspace tests in `target/game26proof` meanwhile. `cargo test` doesn't rebuild `oot_sandbox`.
- **Files:** Bash heredocs choke on apostrophes and eat backslashes: write Python scripts and long
  Markdown with the Write tool. **Never `sed -i` any file here**; in Python read and write bytes and
  keep each file's line endings (check them first).
- **Ids:** check every id against its header. This ROM uses the `_pal` XMLs where they differ
  (`baseroms/gc-eu-mq-dbg/config.yml`). `OOT_VERSION` takes the `>= PAL_1_0` and `>= NTSC_1_1`
  sides, and it isn't `PLATFORM_N64`.
- **Helpers:** at most two at once; each a tight brief, its own worktree under `.claude/worktrees/`
  on a branch from HEAD, its own pack copy (`out/data27_<name>`) and target, and the rules in
  `out/handoff_g11m2/HELPER_RULES.md` (copy them, update the numbers). Merge files you haven't
  touched with `git show <branch>:<path> > <path>`; otherwise `git diff` and `git apply --reject`.
  Remove the worktree, its branch and its pack copy after.
- **Windows:** running `oot` or `oot_sandbox` without `--screenshot`, `--trace` or `--wav` opens a
  window. Ask before opening any window.

**How we work**
- Port function by function from `D:/OOT Modding/oot-main` (52a510f) with the decomp's names. Cite
  constants, mark the game's own bugs "@bug (game)", and order arrays by linkAge, adult first.
- Each new actor: a schema (ADR 0066) and texts (ADR 0079), its Params and Flags paragraphs,
  registration in `lib.rs`'s four places, a name in the editor's `actors.rs`, a character in
  `ootx`'s `characters.rs` for a new skeleton.
- Docs: GAME-11's exit review section and GAME-12's survey; ROADMAP, COVERAGE, BACKLOG and
  `HAND-TESTS-OWED.md` kept in agreement.
- My view of the real game wins over a decomp-faithful visual. If I report something the C or the
  ROM's data does too, show me and ask.
- Commit and push only when I ask.

**Environment:** `CARGO_TARGET_DIR=target/game26`; `OOT_DATA_DIR=out/data27` (absolute for cargo
test); `OOT_SAVE_DIR=out/saves`. Background `&` and `sleep` loops in the foreground are blocked: use
`run_in_background` (a background `until` loop can wait on a file).
