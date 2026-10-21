Next piece of work: **GAME-10 milestone 5, Jabu-Jabu's Belly (Master Quest) to Barinade.**
Read `docs/GAME-10-zoras-waters-and-jabu-jabu.md` first: its survey's "Jabu-Jabu's Belly, Master
Quest" section (Ruto, the tentacles, the way the flags give), the proposal's row 5, "Things to
check", and milestone 4's section (match its parts when you write milestone 5's; its "The way,
settled" corrects the survey). Then ADR 0082. Build it, then give me the hand-test list and
`game-*.bat` starts, and stop for my hand test.

**Where things stand**
- Phase 11 (GAME-10): milestones 1 to 4 are done, hand-tested, committed and pushed (last a024388).
  Milestone 4 ported Jabu-Jabu's draw config (the walls' pulse as site matrices: pack format 30),
  `Bg_Bdan_Switch`, `Bg_Bdan_Objects`, `En_Siofuki`, one-point 3100 with `Camera_Special0`, and the
  six enemies (`En_Bubble`, `En_Bili`, `En_Brob`, `En_Weiyer`, `En_Eiyer`, `En_Rr`).
- COVERAGE: Phase 11 is "in progress", 23 of its 35 actors ported; 206 ported overall.
- 1,633 tests pass, 1 ignored. Goldens: 297 hashes, 224 cases, `out/golden_base33`. Pack format 30
  in `out/data27`; build into `target/game26`.
- Next numbers: ADR 0083, BACKLOG #104, menu entry 160.
- Presets, a chain each run's end is checked against: `jabu-jabu` (Jabu-Jabu's start:
  `Route::JabuFish`'s end), `jabu-room-1` (the slingshot on C-Right, room 0's cow shot: door 1),
  `jabu-room-14` (room 1's yellow switch 0x19: door 2), `jabu-boomerang` (the boomerang's chest:
  treasure flag 6, the spouts down). **Milestone 5 starts from `jabu-boomerang`.**
- Runs: `Route::JabuCow`, `JabuElevator`, `JabuBoomerang`, `JabuDoor14` (`tests/jabu_runs.rs`);
  `JABU_STARTS` in `playthrough.rs` (the hand test's starts, each checked by a test).

**First, my call on BACKLOG #102: give the story's presets the silver scale.** The presets from
`zora-letter` on (`jabu-fish`, `jabu-jabu`, `jabu-room-1`, `jabu-room-14`, `jabu-boomerang`, and
check what else builds on `zoras_domain`'s chain past the diving game) lack the silver scale
(`UPG_SCALE` 2, from the diving game's win), which Ruto's letter needs. Add it where the diving
game's win comes in the story (`zora-letter` and on; not `zora-diving`, `zora-bottle` or
`fairy-magic`), as `Item_Give(ITEM_SCALE_SILVER)` does. Prove any golden it moves (a build with it
taken out at run time) and log it in `golden/README.md`; strike #102.

**Milestone 5's scope** (the proposal's row 5):
- **`En_Ru1` whole** (`ovl_En_Ru1/z_en_ru1.c`, about 2,770 lines with its cutscene scripts): every
  type, Jabu-Jabu's (room 2's first meeting `gRutoFirstMeetingCs`, texts 0x404B and 0x401E, her drop
  through a hole, `INFTABLE_RUTO_MET_FIRST_TIME`; room 3's, carried: 0x404C to 0x404E,
  `INFTABLE_RUTO_LET_LINK_CARRY`, thrown she bounces, in water she bobs and sinks, left behind she
  goes; room 6's `gRutoFoundSapphireCs`, `gRutoObtainingSapphireCs`, `INFTABLE_RUTO_HAS_SAPPHIRE`,
  Big Octo taking her), the fountain's (type 1, `gZorasFountainSapphireCs`), the throne room's
  (type 5), Barinade's warp's (type 0: `Door_Warp1` `WARP_BLUE_RUTO`). She drives
  `Bg_Bdan_Objects`' platform through `BgBdanObjects_GetProperty`/`_SetProperty` (ported, `pub`) and
  weighs down blue switches (`Bg_Bdan_Switch`, `ACTOR_FLAG_CAN_PRESS_SWITCHES`) and the heavy one.
- **Player's Ruto checks:** a door opened while carrying her (`z_player.c:5382-5384`), her cry on his
  falls (`:9472-9478`). The do-action is ported.
- **`En_Ba`** (the tentacles: red 0x0B00 room 8, gray 0x0A02 room 9, green 0x0C01 room 10; only
  `DMG_BOOMERANG`, four hits, their flag), **`En_Bx`** (the electric barriers by colour flag),
  **`Obj_Kibako`** (the crate: presses blue switches, lifted and thrown), **`Obj_Roomtimer`** (shares
  each tentacle's flag). Port each whole, with unit tests that drive its states directly
  (`tests/en_okuta.rs` is the model). No scripted fights.
- The way (the survey's "the way the flags give", corrected by milestone 4): room 1, door 2 →
  room 2 (Ruto's meeting; the cow 0x13 inside boulders: bombs, then the slingshot: door 4) → room 7 →
  room 11 cleared: the stick, lit at the golden torch (lit by the room's clear) → room 7's web
  (`Bg_Ydan_Sp` 0x1FD6, fire) → room 9's gray tentacle (room 7's gray barriers go) → room 7's blue
  0x0D under two boulders (bombs, a crate on it) → room 8's red tentacle (the red barriers go) →
  room 1, door 9 → room 5: the crystal (`Obj_Switch` 0x1403, flag 0x14) → door 0 → room 15 → exit 2
  (`ENTR_JABU_JABU_BOSS_0`). Settle each climb with a search over the floors (milestone 4 rebuilt
  GAME-09's: every floor of each 20-unit cell, walks, climbs to 71, falls, walls by line tests; its
  probe is kept in `out/handoff_m4/probe_jabu.rs`: copy it into `oot_actors/tests` to run it, and remove it after) and the water boxes, only as far as the way needs.
  "Things to check": room 11's lit stick to room 7's web (about 520 units: does it last?), room 5's
  posts up to door 0, room 2's holes.
- **The runs, from debug starts, no fights:**
  - Ruto's first meeting in room 2 (her cutscene, her drop), and the cow bombed free and shot (door 4);
  - room 11's stick lit and carried to room 7's web (or a debug start past it if the stick doesn't
    last: note it);
  - room 5's crystal with the tentacles' flags set by the start, door 0, room 15, the fall to
    Barinade's room.
- `game-jabu.bat` starts beside each new actor and each part of the way (extend `JABU_STARTS` and
  its test), for my hand test: Ruto carried and thrown, the tentacles with the boomerang, the
  barriers, the crate on a switch.

**What milestone 4 found (still applies)**
- Door 14 is shut to Link from room 1 until the boomerang's chest (room 1's spout 0x15C6 stands
  across it; its switch 0x17 is set by nothing in this ROM): door 14 is room 14's way out. The way
  into room 14 is room 3's door 12 (Ruto on the blue switch 0x3C): milestone 8's run, but Ruto's
  carry is yours now.
- Room 1's pit is water to -1248 (`waterBoxes[1]`); the elevator's shaft is walled from 62 under
  it: the west ledge (yellow 0x19) is reached by diving under (`Task::DiveUnder`).
- Room 14's chest ledge (-1073) has no known way up (BACKLOG #103).
- `jabu-fish` lacks `EVENTCHKINF_B5`: on it, `gJabuIntroCs` holds Player (`Play_InCsMode`) for 300
  frames; the later presets have it seen.
- The story's save has the slingshot on no C button; `jabu-room-1` on has it on C-Right. The
  slingshot tasks use whichever C button holds it.
- Jabu's doors stand 80 above their floor; moving collision is whole numbers (`Vec3s`): Link stands
  0 to 1 above a moving platform (the game's; left by my call).
- `eng_math::rad_to_binang` rounds where the C truncates (BACKLOG #101): truncate in new code.

**Helpers (agents)**
- **At most two helpers at once.** About ten at once used up a lot of my usage. A fresh helper with
  a tight brief costs about 230k to 420k tokens. Resuming a stopped one costs 370k to 515k (it
  replays its whole context), so don't resume one; start a fresh one instead.
- Milestone 4's helper rules are a good start (`out/handoff_m4/HELPER_RULES.md`: update its paths
  and numbers): their own worktree under `.claude/worktrees/` on a branch from a snapshot of your
  tree, their own pack folder (`out/data27_<name>`), their own target; run only their own tests,
  `--test schemas` and `cargo test -p layering` (no full workspace run, no goldens); no subagents;
  commit in their worktree and report briefly.
- Merge with `git diff <snapshot>..<commit> > patch` and `git apply --reject`, one patch at a time;
  then the `.rej` files by hand. The usual conflicts are `oot_actors/src/lib.rs`'s four places
  (CRLF), the editor's `actors.rs` and `ootx`'s `characters.rs`: milestone 4's
  `out/handoff_m4/register.py` (a JSON list of actors, anchors found in the file) did them.
- Long local jobs (the test suite, golden checks, imports) cost no tokens: run them as background
  shells.

**Lessons (all still apply)**
- Never script an enemy fight or the long story chain, even if a brief names one. Give a debug start
  beside the enemy (a `game-*.bat` option) and put the fight on the hand-test list.
- Settle reachability only as far as you need to reach and exercise the actors. If it gets deep, note
  it in a line, give a debug start past it, and move on. Coverage matters more than completability.
- Each run's end is checked against the next start's preset (`ends_as_preset`, less a debug start's
  room clears).
- Check a spawn's or start's surroundings before starting a run from it: probe the floor
  (`col.entity_raycast_down`) and the water boxes; a hard stop at an exact coordinate is a wall the
  floor probe can't see (line-test it).
- A `WaitFor(Cond::Switch(..))` can finish before the cutscene the switch starts: follow it with
  `Task::WaitCameras`. Use `Task::Hurry` (full tilt) for run-off jumps; `Task::Texts(closure, step)`
  waits on any condition (an elevator's timer).
- A debug start may mark a room cleared (`start_clears`) so the room's enemies stay out of a run.
- Before building a run on a save's items, check the preset's actual inventory, C buttons and songs.
- In tests, place Link inside the room you loaded, and wait out an entrance cutscene before testing
  anything Player does on his own.
- Player can't spawn during his update: use a `PlayRequest`. Player-side writes from actors go
  through `PlayerIface` (`set_yaw`, `set_player_speed`, `set_pushed`).
- Collider spheres updated in draw go in `draw_update`. Shadows come from `ActorShape_Init`. A
  shadow or anything else drawn from collision must use the captured game frame, not the latest
  collision at 60 Hz (milestone 4's foot shadow).
- A read of another actor's params needs a `// schema: another actor's params read whole` note on
  the line before.
- Tests must not depend on Rand luck. A newly ported actor's Rand calls move other cases' sequences.
- Textures and meshes are baked at import: a bake change needs a re-import
  (`target/game26/release/ootx.exe import` with `OOT_DATA_DIR` set).
- Goldens: prove every changed hash with a build where the cause is taken out
  (`target/game26proof`, run-time env switches; milestone 4's `out/handoff_m4/proof.py` applied
  and reverted the switches: adapt its targets). A trace that changes from a draw-only change is a bug. Record all cases
  (`record --only` rewrites the list), check twice, save the new base (`out/golden_base34`), and
  describe changes in `golden/README.md`. Don't rebuild `target/game26` while a golden check runs on
  its binaries, and don't run the workspace tests while editing what they compile.
- Bash heredocs choke on apostrophes: write Python scripts with the Write tool, and use raw strings
  for Windows paths. Never `sed -i` a CRLF file; in Python read and write bytes and keep each file's
  line endings (`file <path>` first). `rm` with a glob gets blocked: write into a fresh folder
  instead.
- Check every id against its header. This ROM uses `bdan_mq.xml` (`baseroms/gc-eu-mq-dbg/config.yml`).
- Running `oot` or `oot_sandbox` without `--screenshot` or `--trace` opens a window. Ask before
  opening any window. Headless screenshots: `oot_sandbox --entrance ... --script still --frames N
  --screenshot out/...png`.

**How we work**
- Port function by function from `D:/OOT Modding/oot-main` (52a510f) with the decomp's names. Cite
  constants, mark the game's own bugs "@bug (game)", and order arrays by linkAge, adult first.
- Each actor: a schema (ADR 0066), texts (ADR 0079: Ruto has many), Params and Flags paragraphs in
  its module docs, registered in `lib.rs`'s four places, named in the editor's `actors.rs`, a
  character in `ootx`'s `characters.rs` for a new skeleton.
- Docs: GAME-10's milestone 5 section, an ADR (0083) for the milestone's decisions; ROADMAP, COVERAGE
  (`ootx census` then `python scripts/coverage.py`) and BACKLOG kept in agreement.
- My view of the real game wins over a decomp-faithful visual. If I report something the C or the
  ROM's data does too, show me and ask.
- Commit and push only when I ask.

**Environment:** `CARGO_TARGET_DIR=target/game26`; `OOT_DATA_DIR=out/data27` (absolute for cargo
test); `OOT_SAVE_DIR=out/saves`. Background `&` and `sleep` are blocked: use `run_in_background`
and Monitor (or a background `until` loop).
