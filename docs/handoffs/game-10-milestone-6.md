Next piece of work: **GAME-10 milestone 6, Barinade and the Zora's Sapphire (the phase's exit, part 1).**
Read `docs/GAME-10-zoras-waters-and-jabu-jabu.md` first: its survey's "The route a child takes" step 11
(the Sapphire), "Barinade, the warp and the Sapphire", "Things to check" (Barinade's title card), the
proposal's row 6, and milestone 5's section (match its parts when you write milestone 6's). Then
ADR 0083, and ADR 0075 (King Dodongo: the boss pattern to follow). Build it, then give me the
hand-test list and `game-*.bat` starts, and stop for my hand test.

**Where things stand**
- Phase 11 (GAME-10): milestones 1 to 5 are done, committed and pushed (last 5cf3550). **Milestone 5's
  hand test is owed** (I deferred it: `docs/HAND-TESTS-OWED.md`); don't wait for it. If I defer a
  hand test again, add its row there.
- Milestone 5 ported `En_Ru1` whole (every type: Barinade's room's type 0 rises through the
  `WARP_BLUE_RUTO` she spawns as a child; the fountain's type 1 follows `gZorasFountainSapphireCs`),
  `En_Ba`, `En_Bx`, `Obj_Kibako`, `Obj_Roomtimer`, and Player's Ruto checks. `Door_Warp1` has the
  `ruto_warp_state` field she reads and writes, but **not its Ruto states** (`DoorWarp1_RutoWarpIdle`,
  `_RutoWarpOut`, `z_door_warp1.c:558-641`; the port logs them).
- COVERAGE: Phase 11 "in progress", 28 of its 35 ported (`Boss_Va`, `En_Fr`, `En_Kakasi`,
  `En_Kakasi3`, `En_Tp`, `En_Vali`, `Item_Ocarina` left); 211 ported overall.
- 1,664 tests pass, 1 ignored. Goldens: 305 hashes, 229 cases, `out/golden_base34`. Pack format 30
  in `out/data27`; build into `target/game26`.
- Next numbers: ADR 0084, BACKLOG #105, menu entry 160.
- **Milestone 6 starts from `jabu-barinade`** (`Route::JabuCrystal`'s end: Link fallen into
  Barinade's room, `ENTR_JABU_JABU_BOSS_0`). Presets are a chain: each run's end is checked against
  the next start's preset (`ends_as_preset`, less a debug start's room clears).

**Milestone 6's scope** (the proposal's row 6):
- **`Boss_Va`** whole (`ovl_Boss_Va/z_boss_va.c`, 4,011 lines, one actor typed by params: the body,
  three supports, three zappers, ten Bari, three stumps, the door's eight pieces; fight phases at
  3, 9, 15, 18). The boomerang is the fight: the supports and the body take only `DMG_BOOMERANG`
  (`:229-262`); the stunned body the sword, boomerang and stick (`:1161, 1171`). Gohma's pattern
  for the intro and death (`Cutscene_StartManual`, a sub camera, Player's cutscene actions 1, 2, 7,
  8), the title card before `EVENTCHKINF_BEGAN_BARINADE_BATTLE` 0x76 (`:1005-1007, 1047`),
  `NA_BGM_BOSS`, `Flags_SetClear`, `Item_B_Heart`. His own 400-particle effect system (sparks,
  spark balls, the zappers' charge, blood, tumours, gore with a floor raycast, `:3304-3973`), the
  environment's light and fog overrides, `envCtx.screenFillColor`, `EffectSsDtBubble` (ported).
  His death spawns `En_Ru1` type 0 at the nearest of three `sWarpPos` (`:1675-1682`). Re-entered
  cleared: Ruto, or with `EVENTCHKINF_37` a plain warp, and the heart (`:651-663`). His room's six
  pots of hearts.
- **`Door_Warp1`'s Ruto warp** (`DoorWarp1_RutoWarpIdle`, `_RutoWarpOut`, about 84 lines: its sub
  camera; it writes `ruto_warp_state` ENTERED, TALKING, WARPING; `EVENTCHKINF_37`,
  `Item_Give(ITEM_ZORA_SAPPHIRE)`, `ENTR_ZORAS_FOUNTAIN_0` with `CS_INDEX_0`), and the plain warp's
  later visits (`:536-539`).
- **The fountain's cutscene**, `gZorasFountainSapphireCs` (spot08 layer 4, 2,452 frames: Ruto type 1
  on cue 3, `Demo_Effect`'s jewel and light, Navi, texts 0x4025 with its choice, 0x0082,
  `CS_TEXT_ZORA_SAPPHIRE`, 0x4028; terminator 13 to `ENTR_ZORAS_FOUNTAIN_0`): check it plays
  through with what's ported.
- **The runs, from debug starts, no fights:**
  - Barinade's intro (from `jabu-barinade`): the title card, his rise, to the fight's start (the
    battle flag 0x76, the boss music), as `Route::KingDodongo` did;
  - after his defeat (a debug start: his death's end, or the room cleared, whichever the C makes
    sound): the heart container taken, Ruto risen through her warp, "what took you so long", into
    the warp with her, the fountain's cutscene: **the Sapphire** (`ITEM_ZORA_SAPPHIRE`, `EVENTCHKINF_37`).
    Its end is a new preset (the story's: the three stones), milestone 7's start.
- `game-*.bat` starts for my hand test: the fight from its start (supports, zappers, Bari, the
  stunned body), a start after the defeat, and the fountain.
- Tests that drive his states directly (`tests/en_okuta.rs`, `tests/boss_dodongo.rs` are the
  models). No scripted fight.

**What milestone 5 found (still applies)**
- A room marked cleared on a debug start never spawns its room timer (an enemy until its first
  update), so a start also sets the flag a save that beat the room would have.
- Standing still, A puts a carried thing down; a throw needs a run (`Task::ThrowAtRun`, which steers
  at its *last waypoint*: put the waypoints past the release point).
- Floor conveyors (rooms 2 and 7) beat a slow walk: approach webs and the like at a run.
- Small targets: aim a little up their collider (seeds fall; `Task::Shoot` aims at a point).
- In Master Quest Ruto never sets `INFTABLE_RUTO_PLACED_ON_SWITCH` (room 3's switch is 0x3C, she
  wants 0x38). BACKLOG #104: her first step towards the Sapphire jumps 71 in her test (milestone 8).

**Helpers (agents)**
- **At most two helpers at once.** A fresh helper with a tight brief costs about 230k to 500k tokens
  (milestone 5's two: 466k and 496k). Don't resume a stopped one (it replays its whole context);
  start a fresh one.
- `Boss_Va` is one file, so it's one helper's (or split it if you see a clean seam: the effect
  system at `:3304-3973` is the candidate). The main session can do the warp, the cutscene check,
  the runs and the presets.
- Milestone 5's helper rules are in `out/handoff_m5/HELPER_RULES.md` (update paths and numbers):
  their own worktree under `.claude/worktrees/` on a branch from HEAD, their own pack folder
  (`out/data27_<name>`), their own target; only their own tests, `--test schemas` and
  `cargo test -p layering`; no subagents; commit in their worktree and report briefly.
- Merge with `git diff <base>..<commit> > patch` and `git apply --reject`, one patch at a time; then
  the `.rej` files by hand. The usual conflicts are `oot_actors/src/lib.rs`'s four places, the
  editor's `actors.rs` and `ootx`'s `characters.rs` (`out/handoff_m5/register.py`; `out/handoff_m5/libfix.py` is an
  edit that keeps the file's line endings). **`git apply` writes the files it touches with CRLF**:
  scripts that edit them must handle either ending.
- Long local jobs (the test suite, golden checks, imports) cost no tokens: run them as background
  shells.

**Lessons (all still apply)**
- Never script an enemy fight or the long story chain, even if a brief names one. Give a debug start
  beside the enemy (a `game-*.bat` option) and put the fight on the hand-test list.
- Settle reachability only as far as you need to reach and exercise the actors. If it gets deep, note
  it in a line, give a debug start past it, and move on. Coverage matters more than completability.
- Check a spawn's or start's surroundings before starting a run from it: probe the floor
  (`col.entity_raycast_down`) and the water boxes; a hard stop at an exact coordinate is a wall the
  floor probe can't see (line-test it). The floor probe is kept in `out/handoff_m5/probe_jabu.rs`
  (copy it into `oot_actors/tests` to run it, and remove it after); a raycast's first floor can be
  an arch over the real floor.
- A `WaitFor(Cond::Switch(..))` can finish before the cutscene the switch starts: follow it with
  `Task::WaitCameras`. Use `Task::Hurry` (full tilt) for run-off jumps; `Task::Texts(closure, step)`
  waits on any condition.
- Before building a run on a save's items, check the preset's actual inventory, C buttons and songs
  (`jabu-barinade`: the slingshot on C-Right, the bombs on C-Down, the Deku Stick on C-Left, the
  boomerang owned but on no button: put it on one from the item page for the fight's starts).
- In tests, place Link inside the room you loaded, and wait out an entrance cutscene before testing
  anything Player does on his own.
- Player can't spawn during his update: use a `PlayRequest`. Player-side writes from actors go
  through `PlayerIface`.
- Collider spheres updated in draw go in `draw_update`. Shadows come from `ActorShape_Init`. Anything
  drawn from collision must use the captured game frame, not the latest collision at 60 Hz.
- A read of another actor's params needs a `// schema: another actor's params read whole` note on
  the line before.
- Tests must not depend on Rand luck. A newly ported actor's Rand calls move other cases' sequences.
- Textures and meshes are baked at import: a bake change needs a re-import
  (`target/game26/release/ootx.exe import` with `OOT_DATA_DIR` set).
- Goldens: prove every changed hash with a build where the cause is taken out
  (`target/game26proof`, run-time env switches; `out/handoff_m5/proof_m5.py` applies and restores
  them, checking its backups first: adapt its targets; **a proof script must never write a file
  before it has its backup**). A trace that changes from a draw-only change is a bug. Record all
  cases (`record --only` rewrites the list), check twice, save the new base (`out/golden_base35`),
  and describe changes in `golden/README.md`. Don't rebuild `target/game26` while a golden check
  runs on its binaries, and don't run the workspace tests while editing what they compile.
- Bash heredocs choke on apostrophes and eat backslashes: write Python scripts with the Write tool,
  and use raw strings for Windows paths. Never `sed -i` a CRLF file; in Python read and write bytes
  and keep each file's line endings. `grep $'\r'` lies in this shell: check line endings with
  Python. `rm` with a glob gets blocked: remove explicit paths.
- Check every id against its header. This ROM uses `bdan_mq.xml` (`baseroms/gc-eu-mq-dbg/config.yml`).
- Running `oot` or `oot_sandbox` without `--screenshot` or `--trace` opens a window. Ask before
  opening any window. Headless screenshots: `oot_sandbox --entrance ... --script still --frames N
  --screenshot out/...png`.

**How we work**
- Port function by function from `D:/OOT Modding/oot-main` (52a510f) with the decomp's names. Cite
  constants, mark the game's own bugs "@bug (game)", and order arrays by linkAge, adult first.
- Each actor: a schema (ADR 0066), texts (ADR 0079), Params and Flags paragraphs in its module docs,
  registered in `lib.rs`'s four places, named in the editor's `actors.rs`, a character in `ootx`'s
  `characters.rs` for a new skeleton.
- Docs: GAME-10's milestone 6 section, an ADR (0084) for the milestone's decisions; ROADMAP, COVERAGE
  (`ootx census` then `python scripts/coverage.py`), BACKLOG and `HAND-TESTS-OWED.md` kept in
  agreement.
- My view of the real game wins over a decomp-faithful visual. If I report something the C or the
  ROM's data does too, show me and ask.
- Commit and push only when I ask.

**Environment:** `CARGO_TARGET_DIR=target/game26`; `OOT_DATA_DIR=out/data27` (absolute for cargo
test); `OOT_SAVE_DIR=out/saves`. Background `&` and `sleep` loops in the foreground are blocked: use
`run_in_background` and Monitor (or a background `until` loop).
