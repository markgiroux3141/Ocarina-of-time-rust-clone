Next piece of work: **GAME-10 milestone 7, the Ocarina of Time and the Song of Time (the phase's exit, part 2).**
Read `docs/GAME-10-zoras-waters-and-jabu-jabu.md` first: its survey's "The route a child takes" step 12
(the moat), "Things to check" (the moat's depth under `Item_Ocarina`), the proposal's row 7, and
milestone 6's section (match its parts when you write milestone 7's). Then ADR 0084 and ADR 0075.
Build it, then give me the hand-test list and `game-*.bat` starts, and stop for my hand test.

**Where things stand**
- Phase 11 (GAME-10): milestones 1 to 6 are done, committed and pushed (last: "GAME-10: Barinade and
  the Zora's Sapphire"). **Milestones 5's and 6's hand tests are owed** (I deferred them:
  `docs/HAND-TESTS-OWED.md`); don't wait for them. If I defer a hand test again, add its row there.
- Milestone 6 ported `Boss_Va` whole (`boss_va.rs`, `boss_va_effects.rs`), `Door_Warp1`'s Ruto warp
  and one-point 9705; `gZorasFountainSapphireCs` plays through. Its runs: `Route::Barinade`,
  `Route::Sapphire`; presets `barinade-fight`, `barinade-beaten`, **`zora-sapphire`** (the three
  spiritual stones, at Zora's Fountain: this milestone's start).
- COVERAGE: Phase 11 "in progress", 29 of its 35 ported (`En_Fr`, `En_Kakasi`, `En_Kakasi3`,
  `En_Tp`, `En_Vali`, `Item_Ocarina` left); `En_Ganon_Mant` is an older gap (not yet); 212 ported
  overall.
- 1,681 tests pass, 1 ignored. Goldens: 314 hashes, 235 cases, `out/golden_base35`. Pack format 30
  in `out/data27`; build into `target/game26`.
- Next numbers: ADR 0085, BACKLOG #106, menu entry 163.
- Presets are a chain: each run's end is checked against the next start's preset (`ends_as_preset`,
  less a debug start's room clears and anything the hand test's start adds, as an `extra`).

**Milestone 7's scope** (the proposal's row 7; the survey's step 12):
- **`Item_Ocarina`** whole (`ovl_Item_Ocarina/z_item_ocarina.c`, 213 lines, every type: params 0 to
  2 in the cutscenes, frames 220, 881, 897, 906; type 3 the moat's). Type 3 at (299, -136, 884) in
  the moat: killed without `EVENTCHKINF_ZELDA_FLED_CASTLE` (0x80) or with `EVENTCHKINF_43`; spawns
  `Elf_Msg2` 0x3800 (ported); offers `GI_OCARINA_OF_TIME` within 30 across and 50 up or down
  (`Actor_OfferGetItem`); sets 0x43 and switch 3; on the box's close starts
  `gHyruleFieldZeldaSongOfTimeCs` (`z_item_ocarina.c:68-76, 179-199`).
- **`En_Ganon_Mant`** whole (`ovl_En_Ganon_Mant`, 505 lines): Ganondorf's cape in the escape (and
  the nightmare, where `En_Viewer` logs it today: `en_viewer.rs:477`).
- **The cutscenes, checked to play through with what's ported** (as milestone 6 did the fountain's):
  - `gHyruleFieldZeldaEscapeCs` (layer 5, `ENTR_HYRULE_FIELD_0` with 0xFFF1: `Bg_Spot00_Hanebasi`'s
    trigger, ported: Link on the bridge's way, x within 450, z 1080 to 1700, with the three stones
    and not 0x80; `En_Viewer` types 0 to 4, ported; Impa throws `Item_Ocarina` type 0 at frame 845,
    `z_en_viewer.c:378-383`); terminator 33 back to layer 1.
  - `gHyruleFieldZeldaSongOfTimeCs` (from the moat's ocarina); terminator 52 to the Temple of Time,
    `CS_INDEX_7`: `gTempleOfTimeSongOfTimeCs` (Zelda `En_Viewer` 0x0202, the jewels,
    `OCARINA_ACTION_TEACH_TIME` and `PLAYBACK_TIME`: the Song of Time given by the message system,
    `z_message.c:3628`); terminator 53 to `ENTR_HYRULE_FIELD_16`, where `gHyruleFieldGetOoTCs`
    plays. Already ported: the drawbridge's trigger, `En_Viewer`'s escape, terminators 33, 52 and
    53, the three-stones layer, the cutscenes' ocarina actions, every actor of `tokinoma` layer 11.
- **The runs, from debug starts, no fights:**
  - the drawbridge and Zelda's escape: `zora-sapphire` in Hyrule Field short of the bridge (find
    the entrance or `--at` start at the bridge's foot), walked onto the trigger, the escape to its
    terminator, back in layer 1;
  - the Ocarina of Time: from the moat's edge after the escape (the escape's end as a preset), the
    dive to the ocarina (check the moat's water and depth first: the silver scale is in the
    presets), the get-item, the Song of Time's cutscene chain through the Temple of Time to
    `ENTR_HYRULE_FIELD_16`. **Its end is a new preset (the Ocarina of Time and the Song of Time:
    Phase 11's exit met; Phase 12's start).**
- `game-*.bat` starts for my hand test: the field short of the bridge, the moat after the escape,
  the Temple of Time's cutscene, the field after.
- Tests that drive `Item_Ocarina`'s and `En_Ganon_Mant`'s states directly (`tests/ruto_warp.rs`,
  `tests/en_okuta.rs` are the models). No scripted fight.

**What milestone 6 found (still applies)**
- An actor that another's init spawns (`Actor_Spawn` in `Init`) is in the arena only after the next
  update: idle a frame or two before looking for it.
- A run that ends in a boss's room must end as the room is entered: the boss's intro takes Link at
  once, and a standing settle lets it run (it set Barinade's battle flag in `Route::JabuCrystal`).
- A run whose last scene puts Link in water (the fountain's `ENTR_ZORAS_FOUNTAIN_0`) ends on "the
  scene's layer, the transition off, Link free", not on a standing settle (`Route::Sapphire`).
- Barinade's death puts Ruto away from Link, not at the nearest warp spot (the C's loop): check a
  survey's "nearest"/"first" claims against the loop before writing them into a test.
- The fountain's Sapphire cutscene with the Ruby held shows 0x4035 after 0x0082 (both texts).

**Helpers (agents)**
- **At most two helpers at once.** A fresh helper with a tight brief costs about 230k to 600k tokens
  (milestone 6's `Boss_Va`: 614k). Don't resume a stopped one (it replays its whole context); start
  a fresh one.
- This milestone is small (about 720 lines of C). The main session can do it alone; if you use a
  helper, give it `En_Ganon_Mant` (the cape's own simulation) and keep `Item_Ocarina`, the
  cutscene checks, the runs and the presets.
- Helper rules: `out/handoff_m6/HELPER_RULES.md` (update paths and numbers): their own worktree under
  `.claude/worktrees/` on a branch from HEAD, their own pack folder (`out/data27_<name>`, a copy of
  `out/data27`), their own target; only their own tests, `--test schemas` and `cargo test -p
  layering`; no subagents; commit in their worktree and report briefly.
- Merge with `git diff <base>..<commit> > patch` and `git apply --reject`, then the `.rej` files by
  hand. The usual conflicts are `oot_actors/src/lib.rs`'s four places, the editor's `actors.rs` and
  `ootx`'s `characters.rs` (`out/handoff_m5/register.py`; `out/handoff_m5/libfix.py` keeps line
  endings). **`git apply` writes the files it touches with CRLF.** Remove the worktree, its branch and
  its pack copy after the merge.
- Long local jobs (the test suite, golden checks, imports) cost no tokens: run them as background
  shells, and wait with a background `until` loop that also matches the failure lines.

**Lessons (all still apply)**
- Never script an enemy fight or the long story chain, even if a brief names one. Give a debug start
  beside the enemy (a `game-*.bat` option) and put the fight on the hand-test list.
- Settle reachability only as far as you need to reach and exercise the actors. If it gets deep, note
  it in a line, give a debug start past it, and move on. Coverage matters more than completability.
- Check a spawn's or start's surroundings before starting a run from it: probe the floor
  (`col.entity_raycast_down`) and the water boxes; a hard stop at an exact coordinate is a wall the
  floor probe can't see (line-test it). The floor probe is kept in `out/handoff_m6/probe_jabu.rs`
  (copy it into `oot_actors/tests` to run it, and remove it after); a raycast's first floor can be
  an arch over the real floor.
- A `WaitFor(Cond::Switch(..))` can finish before the cutscene the switch starts: follow it with
  `Task::WaitCameras`. Use `Task::Hurry` for run-off jumps; `Task::Texts(cond, step)` presses A
  through texts until any condition holds (it's also the way to wait on a cutscene's end state).
- Before building a run on a save's items, check the preset's actual inventory, C buttons and songs.
- In tests, place Link inside the room you loaded, and wait out an entrance cutscene before testing
  anything Player does on his own.
- Player can't spawn during his update: use a `PlayRequest`. Player-side writes from actors go
  through `PlayerIface`.
- Collider spheres updated in draw go in `draw_update`. Shadows come from `ActorShape_Init`. Anything
  drawn from collision must use the captured game frame, not the latest collision at 60 Hz.
- A read of another actor's params needs a `// schema: another actor's params read whole` note on
  the line before. A schema's `TextLine::when` names the field's name (`Door_Warp1`'s is `params`).
- Tests must not depend on Rand luck. A newly ported actor's Rand calls move other cases' sequences.
- Textures and meshes are baked at import: a bake change needs a re-import
  (`target/game26/release/ootx.exe import` with `OOT_DATA_DIR` set).
- Goldens: prove every changed hash with a build where the cause is taken out
  (`target/game26proof`, run-time env switches; `out/handoff_m6/proof_m6.py` applies and restores
  them, checking its backups first: adapt its targets; **a proof script must never write a file
  before it has its backup**). A trace that changes from a draw-only change is a bug. Record all
  cases (`record --only` rewrites the list), check twice, save the new base (`out/golden_base36`),
  and describe changes in `golden/README.md`. Don't rebuild `target/game26` while a golden check
  runs on its binaries, and don't run the workspace tests while editing what they compile.
- Look at new golden shots headlessly before recording them (`oot_sandbox ... --frames N
  --screenshot out/...png`, then read the PNG).
- Bash heredocs choke on apostrophes and eat backslashes (also inside Python strings in a heredoc:
  use forward slashes in paths there): write longer Python scripts with the Write tool. **Never
  `sed -i` any file here** (in this shell it strips a CRLF file's `\r`s); in Python read and write
  bytes and keep each file's line endings. `grep $'\r'` lies in this shell: check line endings with
  Python. `rm` with a glob gets blocked: remove explicit paths.
- Check every id against its header. This ROM uses the `_pal` XMLs where they differ
  (`baseroms/gc-eu-mq-dbg/config.yml`): Barinade's title card was 128 by 120 there, not 128 by 80.
- Running `oot` or `oot_sandbox` without `--screenshot` or `--trace` opens a window. Ask before
  opening any window. Headless screenshots: `oot_sandbox --entrance ... --script still --frames N
  --screenshot out/...png`.

**How we work**
- Port function by function from `D:/OOT Modding/oot-main` (52a510f) with the decomp's names. Cite
  constants, mark the game's own bugs "@bug (game)", and order arrays by linkAge, adult first.
- Each actor: a schema (ADR 0066), texts (ADR 0079), Params and Flags paragraphs in its module docs,
  registered in `lib.rs`'s four places, named in the editor's `actors.rs`, a character in `ootx`'s
  `characters.rs` for a new skeleton.
- Docs: GAME-10's milestone 7 section, an ADR (0085) for the milestone's decisions; ROADMAP, COVERAGE
  (`ootx census` then `python scripts/coverage.py`), BACKLOG and `HAND-TESTS-OWED.md` kept in
  agreement.
- My view of the real game wins over a decomp-faithful visual. If I report something the C or the
  ROM's data does too, show me and ask.
- Commit and push only when I ask.

**Environment:** `CARGO_TARGET_DIR=target/game26`; `OOT_DATA_DIR=out/data27` (absolute for cargo
test); `OOT_SAVE_DIR=out/saves`. Background `&` and `sleep` loops in the foreground are blocked: use
`run_in_background` and Monitor (or a background `until` loop).
