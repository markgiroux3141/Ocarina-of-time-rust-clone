# Handoff: Phase 12's survey (the Temple of Time and adult Link)

Paste the prompt below into a fresh session (from the repo root). Written 2026-10-23, after GAME-10
milestone 9 (35c4c80, ADR 0087), the last of Phase 11's milestones, and Phase 11's exit review.

---

## Prompt

Next piece of work: **Phase 12's survey** (the Temple of Time, the market's ruins, the Chamber of
the Sages; adult Link). Survey, then write a proposal and stop at it for my calls. Don't build
anything yet.

**Where things stand**
- **Phase 11 (GAME-10) is done**, committed and pushed (last: "GAME-10: the frogs and the
  scarecrows", 35c4c80). Its exit review is the last section of
  `docs/GAME-10-zoras-waters-and-jabu-jabu.md`: read it first, especially "Phase 12's start" and
  "What's left, and where". Milestone 9's hand test was deferred (my call); it's a row in
  `docs/HAND-TESTS-OWED.md`, the only open one. If I defer another, add its row there.
- **Phase 12 starts from the `ocarina-of-time` preset** (checked against `Route::OcarinaOfTime`'s end,
  Hyrule Field at `ENTR_HYRULE_FIELD_16`). With it Link has: the Kokiri Sword on B, the Deku and
  Hylian Shields, the Kokiri Tunic and Boots, the silver scale; Deku Sticks (1) on C-Left, the
  boomerang on C-Down, the Fairy Slingshot (37 seeds) on C-Right, bombs (15), the Ocarina of Time on
  no button, an empty bottle, Zelda's Letter; the three spiritual stones; Zelda's Lullaby, Epona's,
  Saria's, the Sun's Song and the Song of Time (no Song of Storms); six hearts and two pieces; magic
  level 1; 16 rupees. Check these against the preset yourself before relying on them.
  `ocarina-c` is a debug preset (the ocarina on C-Down, the Song of Storms) if a start wants one.
- COVERAGE: 221 ported overall; Phase 12's row lists 4 placed actors, none ported (`En_Nb`, `En_Rl`,
  `En_Ru2`, `Obj_Mure3`). That row only counts actors whose first scene is Phase 12's. The Temple of
  Time's actors behind the Door of Time (BACKLOG #37: `Bg_Toki_Swd`, `Demo_6K` type 16, `En_Xc` type
  9, `Bg_Spot02_Objects` type 4) and every adult placeholder chosen at init in earlier phases count
  elsewhere. Survey them anyway.
- 1,771 tests pass, 1 ignored. Goldens: 337 hashes, 251 cases, `out/golden_base40`. **Pack format
  32** in `out/data27`; build into `target/game26` (`scripts/run/_env.bat`).
- Next numbers: ADR 0088, BACKLOG #110, menu entry 169. The new phase doc is `docs/GAME-11-<name>.md`.

**The survey** (follow ROADMAP's "How a phase starts and how a milestone is done", and model the
doc on `docs/GAME-10-zoras-waters-and-jabu-jabu.md`'s survey, proposal and calls sections):
- ROADMAP's outline row 12: the Door of Time (#37), the Master Sword, **adult Link throughout**
  (models, items, the save's age), time travel back and forth, the Prelude of Light, the warp
  songs. Exit: adult Link out of the Temple.
- **The scenes and their layers** (`ootx scene-info --scene <name> --layer <n>`): the Temple of Time
  (`tokinoma`) with its adult and cutscene layers, the Chamber of the Sages, the market's ruins
  (the adult market and its entrance), the adult Hyrule Field around the Temple. Child layers 0 and 1,
  adult 2 and 3, cutscene layers 4 and up.
- **The route:** from `ocarina-of-time` to the Door of Time (the Song of Time with the three
  stones), the Master Sword, the Chamber of the Sages (Rauru, the Light Medallion), the adult
  awakening, Sheik and the Prelude of Light, out of the Temple as an adult; and back to child
  (the sword returned). Name each cutscene, flag, text and entrance from the C.
- **Adult Link is the big system:** list what Player has for an adult already (the spikes ported
  Player from `z_player.c` for both ages; check what an `--adult` start does now: models,
  animations, `ageProperties`, items, the B button) and what's missing: the adult's model and
  equipment draws, the Master Sword and its swap with the child's sword, the age switch in the save
  (`linkAge`, the equipment and C buttons saved per age, `Inventory_SwapAgeEquipment`), the
  scene-layer choice by age, each ported actor's adult branches. Size each in lines of C.
- **The adult's placeholders chosen at init in Phases 6 to 11** (search the ported actors for the
  adult branches they log or stub): which this phase must make whole, which belong to the adult
  phases later (13 to 20, by ROADMAP's outline).
- **What's already ported and reused** (time travel's cutscenes that are already in? the warp songs'
  `Demo_Kankyo` and BACKLOG #26; Player's ocarina; `En_Viewer`; the Temple's child layer).
- **Things to check** for me, and the **sizes** in lines of C.

Then a **proposal**: milestones in the story's order, the exit as early as it can be, a run for each
(short debug starts; no fights scripted), and the calls that are mine (scope of adult Link this
phase versus later; which adult placeholders get made whole now; any BACKLOG to fold in). Write it
into the new GAME-11 doc, update ROADMAP's Phase 12 entry to "surveyed and proposed", and stop for my
calls.

**What Phase 11 found (still applies)**
- Settle a way with the real game's walkthroughs when the geometry is unclear (Wikibooks' pages
  fetch; StrategyWiki and ZeldaDungeon refuse). Probe floors and water before writing a run
  (`out/handoff_m9/probe_m9.rs` is the latest probe: copy it into `oot_actors/tests`, run with
  `--ignored` and `PROBE_OUT` set to an absolute path, and remove it after).
- A debug start can land beside a forced text (`En_Wonder_Talk2` mode 1): read it first in a run.
  An entrance with an intro cutscene sets its flag in the run (`ends_as_preset` then needs it in the
  preset, or as an `extra`).
- `lastOcarinaButtonIndex` lives one frame (`Message_Update` resets it): read it on the frame it's set.
- Actors that listen for the ocarina mark Link within a range (`PLAYER_STATE2_23`); check that range
  before placing a start.
- The run tasks: `Task::OcarinaNotes` (notes one at a time while a listener's box is ready),
  `Task::OcarinaHeard` (a song, done when heard), `Task::TextsFor(Cond, Step)`, `Task::WalkUntil`,
  `Task::DiveFor`, `Task::LiftWho`, `Task::Texts`, `Task::Hurry`, `Task::WaitCameras`.
- Presets are a chain: each run's end is checked against the next start's preset (`ends_as_preset`,
  less a debug start's room clears and anything the start adds, as an `extra`).
- Diagnostic log lines (silent unless `RUST_LOG` is set): `[target]`, `[camera]`, `[frame]` at info,
  `[sfx] start` at debug; `OOT_PAD_DEBUG=1`. PowerShell's `2>` writes UTF-16: decode before grepping.
- Player has two `Player_UpdateHostileLockOn`s: `player_update_hostile_lock_on_env(env)` is the C's;
  never call the clearing half alone.

**Helpers (when the build starts, not for the survey)**
- At most two helpers at once, each with a tight brief, their own worktree under `.claude/worktrees/`
  on a branch from HEAD, their own pack copy (`out/data27_<name>`), their own target; rules in
  `out/handoff_m9/HELPER_RULES.md` (copy and update the numbers). Keep framework work (Player, the
  save, the message box, the ocarina) in the main session.
- Merging: `git diff <base>..<branch> > patch`, `git apply --reject`, the `.rej` files by hand (the
  usual ones: `oot_actors/src/lib.rs`'s PROFILES and `register` lines, `onepoint.rs`'s doc list and
  last arm, the editor's `actors.rs`, `ootx`'s `characters.rs`). `git apply` writes CRLF: put each file
  back as it was (`lib.rs`, `characters.rs`, `actors.rs`, `save.rs`, `item.rs`, `effect/mod.rs`,
  `player.rs`, `play_scene.rs`, `docs/BACKLOG.md` and the `.bat` files are CRLF; most others LF).
  Remove the worktree, its branch and its pack copy after.
- An old worktree from an earlier session is still there (`.claude/worktrees/agent-a7f58788f8f299170`
  and several `worktree-agent-*` branches): leave them unless I say.

**Lessons (all still apply)**
- Never script an enemy fight or the long story chain, even if a brief names one. Give a debug start
  beside it (a `game-*.bat` option) and put it on the hand-test list.
- Settle reachability only as far as you need to reach and exercise the actors. Coverage matters
  more than completability.
- Before building a run on a save's items, check the preset's actual inventory, C buttons and songs.
- In tests, place Link inside the room you loaded and wait out an entrance cutscene first.
- Player can't spawn during his update (`PlayRequest`); Player-side writes from actors go through
  `PlayerIface`. Collider spheres updated in draw go in `draw_update`. Shadows come from
  `ActorShape_Init`.
- Tests must not depend on Rand luck. A newly ported actor's Rand calls move other cases' sequences.
- Textures and meshes are baked at import: a bake change needs a re-import; a change to what a bake
  stores needs `FORMAT_VERSION` bumped.
- Goldens: prove every changed hash with a build where the cause is taken out (`target/game26proof`,
  a run-time env switch). A trace that changes from a draw-only change is a bug. Record all cases,
  check twice, save the new base (`out/golden_base41`), describe changes in `golden/README.md`. Don't
  rebuild `target/game26` while a golden check runs on its binaries; use `target/game26proof` for
  tests meanwhile. `cargo test` doesn't rebuild `oot_sandbox`: rebuild it before a sandbox run.
- Bash heredocs choke on apostrophes and eat backslashes: write Python scripts with the Write tool.
  **Never `sed -i` any file here**; in Python read and write bytes and keep each file's line endings.
- Check every id against its header. This ROM uses the `_pal` XMLs where they differ
  (`baseroms/gc-eu-mq-dbg/config.yml`); `OOT_VERSION` takes the `>= PAL_1_0` and `>= NTSC_1_1` sides,
  and it isn't `PLATFORM_N64`.
- Running `oot` or `oot_sandbox` without `--screenshot`, `--trace` or `--wav` opens a window. Ask
  before opening any window.

**How we work**
- Port function by function from `D:/OOT Modding/oot-main` (52a510f) with the decomp's names. Cite
  constants, mark the game's own bugs "@bug (game)", and order arrays by linkAge, adult first.
- Each actor: a schema (ADR 0066), texts (ADR 0079), Params and Flags paragraphs, registered in
  `lib.rs`'s four places, named in the editor's `actors.rs`, a character in `ootx`'s `characters.rs`
  for a new skeleton.
- Docs: the GAME-11 doc; ROADMAP, COVERAGE (`ootx census`, then `python scripts/coverage.py`),
  BACKLOG and `HAND-TESTS-OWED.md` kept in agreement.
- My view of the real game wins over a decomp-faithful visual. If I report something the C or the
  ROM's data does too, show me and ask.
- Commit and push only when I ask.

**Environment:** `CARGO_TARGET_DIR=target/game26`; `OOT_DATA_DIR=out/data27` (absolute for cargo
test); `OOT_SAVE_DIR=out/saves`. Background `&` and `sleep` loops in the foreground are blocked: use
`run_in_background` (a background `until` loop can wait on a file).
