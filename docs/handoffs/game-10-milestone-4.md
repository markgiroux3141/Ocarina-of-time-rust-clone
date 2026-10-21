# Handoff: GAME-10 milestone 4, Jabu-Jabu's Belly to the boomerang

Paste the prompt below into a fresh session (from the repo root). Written 2026-10-21, after GAME-10
milestones 1 to 3 (f9c7e29, 13844a7).

---

## Prompt

Next piece of work: **GAME-10 milestone 4, Jabu-Jabu's Belly (Master Quest) to the boomerang.**
Read `docs/GAME-10-zoras-waters-and-jabu-jabu.md` first: its survey's "Jabu-Jabu's Belly, Master
Quest" section, the proposal's row 4, "Things to check", and milestones 1 to 3's sections (match
their parts when you write milestone 4's). Build it, then give me the hand-test list and `game-*.bat`
starts, and stop for my hand test.

**Where things stand**
- Phase 11 (GAME-10): milestones 1 (water and bottles, ADR 0081), 2 and 3 (the river, the Domain,
  the lake, the fountain: King Zora and Jabu-Jabu's mouth) are done, hand-tested, committed and
  pushed (f9c7e29, 13844a7). `Route::JabuFish` ends inside Jabu-Jabu (`SCENE_JABU_JABU`, room 0).
- **Already ported for this milestone:** `En_Boom` and Player's boomerang (`func_80835800` to
  `func_80835C08`, the open left hand while thrown, `tests/boomerang.rs`); `Effect_Ss_Dt_Bubble`
  and `Lightning`; the view's distortion system (`oot_game::view`, `View_SetDistortion*`, applied
  to the projection), which Jabu's draw config drives.
- COVERAGE: Phase 11 is "in progress", 14 of its 35 actors ported; 197 ported overall.
- 1,566 tests pass, 1 ignored. Goldens: 287 hashes, 217 cases, `out/golden_base32`. Pack format 29
  in `out/data26`; build into `target/game26`.
- Next numbers: ADR 0082, BACKLOG #101, menu entry 157.
- Presets: `fairy-magic` (Phase 11's start); `zoras_domain()` (fairy-magic plus `EVENTCHKINF_39`,
  the falls opened, and `_A4`) under `zora-bottle`, `zora-diving`, `zora-letter`, `jabu-fish` and
  `boomerang`. Jabu-Jabu's start will be `jabu-fish`'s save after the run: the flag 0x3A set and
  the bottle empty. Check with `ends_as_preset` against `Route::JabuFish`'s end.

**Milestone 4's scope** (the proposal's row 4):
- `SDC_JABU_JABU` (`Scene_DrawConfigJabuJabu`, `z_scene_table.c:1585-1661`): the scrolls and the
  view's breathing distortion every frame. Add it to the importer's draw-config comparison
  (`crates/import/oot_import/tests/pack.rs`).
- `Bg_Bdan_Switch` (blue, yellow, tall yellow), `Bg_Bdan_Objects` whole (Big Octo on the octo
  platform a placeholder until milestone 8), `En_Siofuki` (the spouts), one-point 3100.
- The enemies on the way: `En_Bubble`, `En_Bili`, `En_Brob`, `En_Weiyer`, `En_Eiyer`, `En_Rr`.
  Port each whole, with unit tests that drive its states directly (`tests/en_okuta.rs` is the
  model). No scripted fights.
- Settle Jabu's climbs with the collision search (GAME-09 milestone 3's), only as far as the way
  needs: room 1's elevator and its ledges, room 14's spouts to the boomerang's ledge.
- Look at Jabu's 106 floor-conveyor polygons ("Things to check").
- The runs, from debug starts, no fights:
  - room 0's west cow shot with the slingshot (door 1);
  - room 1's elevator, yellow switch 0x19, door 14;
  - room 14 marked cleared (`start_clears`): the boomerang's chest.
- `game-jabu.bat` starts beside each enemy and each part of the way, for my hand test.

**Helpers (agents)**
- **At most two helpers at once.** About ten at once used up a lot of my usage. A fresh helper with
  a tight brief costs about 230k tokens. Resuming a stopped one costs 370k to 515k (it replays its
  whole context), so don't resume one; start a fresh one instead.
- Give each helper a snapshot of your working tree: a commit made with a temporary index and
  `git commit-tree`, on a `snap-*` branch, in a worktree under `.claude/worktrees/`. Tell it:
  - run only its own tests, `--test schemas` and `cargo test -p layering` (no full workspace run,
    no goldens);
  - no subagents;
  - its own pack folder (`out/data26_<name>`);
  - commit in its worktree, and report briefly.
- Merge with `git diff <snapshot>..<commit> > patch` and `git apply --reject`. Then resolve the
  `.rej` files by hand, one patch at a time: a second apply overwrites the first's `.rej` files.
  The usual conflicts are `oot_actors/src/lib.rs`'s four places, `scene_table.rs`'s `PORTED`, the
  pack test's draw-config list, `characters.rs` and the editor's `actors.rs`. `lib.rs` is CRLF.
- Long local jobs (the test suite, golden checks, imports) cost no tokens: run them as background
  shells.

**Lessons (all still apply)**
- Never script an enemy fight or the long story chain, even if a brief names one. Give a debug start
  beside the enemy (a `game-*.bat` option) and put the fight on the hand-test list.
- Settle reachability only as far as you need to reach and exercise the actors. If it gets deep, note
  it in a line, give a debug start past it, and move on. Coverage matters more than completability.
- Each run's end is checked against the next start's preset (`ends_as_preset`). That's how
  milestones 1 to 3 found the missing heart container and `EVENTCHKINF_39`.
- Check a spawn's surroundings before starting a run from it: Zora's River's spawn 2 is behind
  the falls, and King Zora's throne faces away from his room's spawn. Probe the floor with
  `col.entity_raycast_down` in a throwaway test, and give the route a start (`Route::start`).
- A `WaitFor(Cond::Switch(..))` can finish before the cutscene the switch starts: follow it with
  `Task::WaitCameras`. Use `Task::Hurry` (full tilt) for run-off jumps.
- A debug start may mark a room cleared (`start_clears`) so the room's enemies stay out of a run.
- Before building a run on a save's items, check the preset's actual inventory and songs. Don't
  trust a brief's summary of it.
- In tests, place Link inside the room you loaded, or that room's objects never load.
- Player can't spawn during his update: use a `PlayRequest` with a predicted handle
  (`SpawnHeldExplosive`, `SpawnBoomerang`). Player-side writes from actors go through
  `PlayerIface` (`set_yaw` is new).
- Collider spheres updated in draw go in `draw_update`. Shadows come from `ActorShape_Init`.
- A read of another actor's params needs a `// schema: another actor's params read whole` note (the
  schemas test).
- Tests must not depend on Rand luck. A newly ported actor's Rand calls move other cases' sequences.
- An ocarina route's test needs the audio side (`tests/zora_runs.rs`'s `run`).
- Textures and meshes are baked at import: a bake change needs a re-import
  (`target/game26/release/ootx.exe import` with `OOT_DATA_DIR` set).
- Goldens: prove every changed hash with a build where the cause is taken out
  (`target/game26proof`, run-time env switches). A trace that changes from a draw-only change is a
  bug. Record and save the new base (`out/golden_base33`), and describe trace changes field by field
  in `golden/README.md`. Don't rebuild `target/game26` while a golden check runs on its binaries.
- Bash heredocs choke on apostrophes: write Python scripts with the Write tool, and use raw strings
  for Windows paths. Never `sed -i` a CRLF file; in Python read and write bytes and keep each file's
  line endings (`file <path>` first). `rm` with a glob gets blocked: write into a fresh folder
  instead.
- Check every id against its header. This ROM uses `bdan_mq.xml` (`baseroms/gc-eu-mq-dbg/config.yml`).
- Running `oot` or `oot_sandbox` without `--screenshot` or `--trace` opens a window. Ask before
  opening any window. Headless screenshots: `oot_sandbox --entrance ... --script idle --frames N
  --screenshot out/...png`.

**How we work**
- Port function by function from `D:/OOT Modding/oot-main` (52a510f) with the decomp's names. Cite
  constants, mark the game's own bugs "@bug (game)", and order arrays by linkAge, adult first.
- Each actor: a schema (ADR 0066), texts (ADR 0079), Params and Flags paragraphs in its module docs,
  registered in `lib.rs`'s four places, named in the editor's `actors.rs`.
- Docs: GAME-10's milestone 4 section, an ADR (0082) for the milestone's decisions; ROADMAP, COVERAGE
  (`ootx census` then `python scripts/coverage.py`) and BACKLOG kept in agreement.
- My view of the real game wins over a decomp-faithful visual. If I report something the C or the
  ROM's data does too, show me and ask.
- Commit and push only when I ask.

**Environment:** `CARGO_TARGET_DIR=target/game26`; `OOT_DATA_DIR=out/data26` (absolute for cargo
test); `OOT_SAVE_DIR=out/saves`. Background `&` and `sleep` are blocked: use `run_in_background` and
Monitor.
