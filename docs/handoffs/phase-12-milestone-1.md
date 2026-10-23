Next piece of work: **Phase 12, milestone 1: adult Link and the time machine** (GAME-11). Build
it, run it, write it up, then stop for my hand test. Commit only when I ask.

**Where things stand**
- **Phase 12 is surveyed and decided.** Read `docs/GAME-11-the-temple-of-time-and-adult-link.md`
  first: the survey (the route, the age switch, what Player has and lacks), the proposal and the
  calls. I took every recommendation (2026-10-23).
- The three milestones: 1 adult Link and the time machine; 2 the Door of Time to the Chamber of
  the Sages and out (the exit); 3 Sheik's Prelude, the sword put back, the warp.
- **Phase 11 (GAME-10) is done and pushed** (35c4c80). Milestone 9's hand test is owed
  (`docs/HAND-TESTS-OWED.md`, the only open row); if I defer another, add its row there.
- The phase starts from the **`ocarina-of-time`** preset. Read from the preset (2026-10-23): the
  Kokiri Sword on B; sticks, boomerang, slingshot on C; bombs 15; the Ocarina of Time on no
  button; an empty bottle; Zelda's Letter; the Deku and Hylian Shields (Hylian worn); the silver
  scale, the Goron's Bracelet; the three stones and five songs (no Storms); six hearts; magic 1;
  16 rupees. `childEquips` and `adultEquips` are empty. `EVENTCHKINF_A7` is clear (the Temple's
  intro plays on the first normal entry).
- 1,771 tests pass, 1 ignored. Goldens: 337 hashes, 251 cases, `out/golden_base40`. **Pack
  format 32** in `out/data27`; build into `target/game26` (`scripts/run/_env.bat`).
- Next numbers: ADR 0088, BACKLOG #110, menu entry 169.

**Milestone 1's scope** (GAME-11's proposal row 1; line numbers are the decomp's):
- **The age switch:**
  - `Inventory_SwapAgeEquipment` (`z_parameter.c:1210-1317`). Its first adult swap gives the
    Master Sword on B, the Hylian Shield and the Kokiri Tunic and Boots.
  - `play->linkAgeOnLoad` (`z_scene.c:277`), and `Play_Destroy`'s swap while the old age is
    still current (`z_play.c:300-303`).
  - `Player_Destroy`'s `linkAge = linkAgeOnLoad` (`z_player.c:12362`).
  - The 13 destinations that set it: `oot_game/src/cutscene.rs:1048`'s `age_not_ported` calls
    (this phase's are 8 and 25).
  - The order matters: the swap runs before the actors are destroyed (`z_play.c:305`).
- **`Bg_Toki_Swd` whole** (191 lines; its three scripts are in the pack already) and
  **`Door_Toki`** (63, `Demo_Kankyo` 0xD's child). Close BACKLOG #37 and #57.
- **Player's time travel:**
  - the adult's `Bg_Toki_Swd` branch in `Player_ActionHandler_2` (`z_player.c:7414-7428`, noted
    at `player.rs:11210-11214`);
  - `Player_PutSwordInHand` (10557-10574);
  - start mode 1, `Player_StartMode_TimeTravel` (10576-10599);
  - `Player_Action_TimeTravelEnd` (13945-13985);
  - cutscene mode 18's update `func_80851A50` with `D_808551A4` and `D_808551AC`
    (15524-15569), and its left-hand override (the `Loadout` has none yet);
  - mode 30's update `func_80851D80` (15616-15624).
- **The adult's draw details** (`z_player_lib.c`): the gauntlets' plates (1131-1150), the iron
  and hover boots' lists (1079-1087, 1152-1157), the tunic's colour at draw time (1125-1126; now
  baked as tunic 0's, `player.rs:12855`), the broken knife's left hand (1364-1365).
- **At the milestone's start:**
  - Fix the stale docs: BACKLOG #37 (`Bg_Spot02_Objects` 4 is ported), #53 to Phase 17, #55 to
    Phase 19; GAME-10's "What's left" (the frozen falls are ported); `demo_kankyo.rs:20`;
    `obj_timeblock.rs:251`.
  - Apply the coverage corrections (GAME-11's list) with the coverage refresh.

**The run** (a short debug start; no fights):
- A new preset `master-sword`: `ocarina-of-time` with the door open (0x4B), the sword revealed
  (0x4F) and obtained (0x45). Link stands in room 0 as a child.
- **The pull:** `gPullMasterSwordCs`, then terminator 8 (0x45 set: the age flips). The equips
  are swapped and the adult arrives at `ENTR_TEMPLE_OF_TIME_2` by `TimeTravelEnd`, the Master
  Sword on B.
- **The way back:** with `EVENTCHKINF_55` added (as an `extra` or a second preset), he puts the
  sword back (`gPlaceMasterSwordCs`). He arrives a child with the Kokiri Sword on B, and both
  ages' equips are kept.
- Tests from the C for the swap (B, C buttons, equipment each way), the arrival's frames, and the
  draw details. A `game-*.bat` debug start for my hand test (child at the pedestal; adult with
  0x55 at the pedestal).
- Check "Things to check" in GAME-11 that touch this milestone: whether a child can walk through
  the closed Door of Time now (`Door_Toki` has no collision yet), the pull's dimming
  (`SDC_TEMPLE_OF_TIME` reading `roomCtx.drawParams[0]`), the first adult's equips.

**Helpers:** this milestone is framework (Player, the save, the cutscene system): keep it in the
main session. If there's room, one helper may start milestone 2's actors in parallel:
- `En_Rl` whole;
- `Demo_6K` types 14 to 19, the others placeholders chosen at init;
- `En_Xc` types 9 and 1 and the pedestal block, the others placeholders.

Give it a tight brief, its own worktree under `.claude/worktrees/` on a branch from HEAD, its own
pack copy (`out/data27_<name>`) and its own target. The rules are in
`out/handoff_m9/HELPER_RULES.md`: copy it and update the numbers. At most two helpers at once.

**Merging:** `git diff <base>..<branch> > patch`, `git apply --reject`, then the `.rej` files by
hand. The usual ones: `oot_actors/src/lib.rs`'s PROFILES and `register` lines, `onepoint.rs`'s doc
list and last arm, the editor's `actors.rs`, `ootx`'s `characters.rs`. `git apply` writes CRLF, so
put each file back as it was: `lib.rs`, `characters.rs`, `actors.rs`, `save.rs`, `item.rs`,
`effect/mod.rs`, `player.rs`, `play_scene.rs`, `docs/BACKLOG.md` and the `.bat` files are CRLF;
most others LF. Remove the worktree, its branch and its pack copy after. Leave the old worktree
(`.claude/worktrees/agent-a7f58788f8f299170`) and the `worktree-agent-*` branches unless I say.

**What the survey found (applies here)**
- **Player's cutscene modes** are table-driven (`GameData::cs_mode_starts/updates` from
  `D_80854B18`/`D_80854E50`). Named functions go through `cs_mode_func` (`player.rs:10237`). Of
  the functions this phase's scripts cue, only `func_80851A50` and `func_80851D80` are missing.
  Map cues with the C's `sCueToCsActionMap` (= the port's `S_CUE_TO_CS_ACTION_MAP`).
- **Terminator 8** (`cutscene.rs:1050-1060`): the first time it goes to the cutscene map
  (`0xFFF3`); later it goes to `ENTR_TEMPLE_OF_TIME_2` with a white fill. Spawn 2's params 0x0100
  are start mode 1.
- **`--adult --entrance`** makes a new save (`Sram_InitNewSave`: nothing on B, nothing worn). The
  scene-only sandbox uses `Sram_InitDebugSave`. Adult starts come from presets.
- **`Bg_Toki_Swd` offers itself** only to a child, or an adult with `EVENTCHKINF_55`
  (`z_bg_toki_swd.c:116`). It hides for an adult at init, and shows again when the sword is put
  back (`:148-155`). On layer 5 it drives `drawParams[0]`.
- **The equipment page is a stand-in** (`save.rs:1913-1935`), left for Phase 20 (my call).

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
  `Task::WaitCameras`.
- Presets are a chain: each run's end is checked against the next start's preset
  (`ends_as_preset`, less a debug start's room clears and anything the start adds, as an
  `extra`).
- Diagnostic log lines (silent unless `RUST_LOG` is set): `[target]`, `[camera]`, `[frame]` at
  info, `[sfx] start` at debug; `OOT_PAD_DEBUG=1`. PowerShell's `2>` writes UTF-16: decode before
  grepping.

**Lessons (all still apply)**
- **Fights and routes:** never script an enemy fight or the long story chain, even if a brief
  names one. Give a debug start beside it (a `game-*.bat` option) and put it on the hand-test
  list. Settle reachability only as far as you need to reach and exercise the actors.
- **Saves:** before building a run on a save's items, check the preset's actual inventory, C
  buttons and songs. In tests, place Link inside the room you loaded and wait out an entrance
  cutscene first.
- **Player and actors:** Player can't spawn during his update (`PlayRequest`); Player-side
  writes from actors go through `PlayerIface`. Collider spheres updated in draw go in
  `draw_update`. Shadows come from `ActorShape_Init`.
- **Rand:** tests must not depend on Rand luck. A newly ported actor's Rand calls move other
  cases' sequences.
- **Bakes:** textures and meshes are baked at import. A bake change needs a re-import; a change to
  what a bake stores needs `FORMAT_VERSION` bumped. The tunic colour at draw time may need either.
- **Goldens:**
  - Prove every changed hash with a build where the cause is taken out (`target/game26proof`, a
    run-time env switch). A trace that changes from a draw-only change is a bug.
  - Record all cases, check twice, save the new base (`out/golden_base41`), and describe the
    changes in `golden/README.md`.
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
- Each actor gets:
  - a schema (ADR 0066) and texts (ADR 0079);
  - its Params and Flags paragraphs;
  - registration in `lib.rs`'s four places;
  - a name in the editor's `actors.rs`;
  - a character in `ootx`'s `characters.rs` if it brings a new skeleton.
- Docs: GAME-11's milestone section; ROADMAP, COVERAGE (`ootx census`, then
  `python scripts/coverage.py`), BACKLOG and `HAND-TESTS-OWED.md` kept in agreement. An ADR for
  the age switch (0088).
- My view of the real game wins over a decomp-faithful visual. If I report something the C or the
  ROM's data does too, show me and ask.
- The milestone is done by ROADMAP's "How a phase starts and how a milestone is done". End with
  the check commands and a concrete hand-test list (keys and places).
- Commit and push only when I ask.

**Environment:** `CARGO_TARGET_DIR=target/game26`; `OOT_DATA_DIR=out/data27` (absolute for cargo
test); `OOT_SAVE_DIR=out/saves`. Background `&` and `sleep` loops in the foreground are blocked:
use `run_in_background` (a background `until` loop can wait on a file).
