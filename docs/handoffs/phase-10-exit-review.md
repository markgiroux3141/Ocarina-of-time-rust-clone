# Handoff: Phase 10's exit review, then Phase 11's survey

Paste the prompt below into a fresh session (from the repo root). Written 2026-10-20, after GAME-09
milestone 5 (0d97540, ADR 0080), the last of Phase 10's milestones.

---

## Prompt

Next piece of work: **Phase 10's exit review** (GAME-09's last section), then **Phase 11's survey**.
Do the review, show it to me, then go on to the survey and stop at its proposal for my calls.

**Where things stand**
- Phase 10 (GAME-09, `docs/GAME-09-death-mountain-and-dodongos-cavern.md`) has all six milestones
  built, hand-tested, committed and pushed: 1 bombs (ec22e9f), 2 the trail and Goron City (3050f4f),
  3 the cavern's route (430e1e2), 4 King Dodongo and the Ruby, **the exit** (1e7de8d), 6 the summit
  and magic (e626b1e, done before 5), 5 the rest of the cavern (0d97540). The editor's actors tab
  (796c9af, 7266d53, 0d8f2aa: ADRs 0077 to 0079) came in between and isn't the phase's.
- COVERAGE: Phase 10's scenes place 33 actors, all 33 ported. `scripts/coverage.py`'s `SCENES` table
  still says "in progress" for phase 10.
- 1,433 tests pass, 1 ignored. Goldens: 273 hashes, 207 cases, `out/golden_base30`. Pack format 29 in
  `out/data26` (`scripts/run/_env.bat`); build into `target/game26`.
- Next numbers: ADR 0081, BACKLOG #100, menu entry 152.
- The last presets: `fairy-magic` (milestone 6's end), `cavern-stairs`, `cavern-rooms` (milestone
  5's end: `fairy-magic` plus room 2's stairs down and room 14's chest), `cavern-song-of-time` (a
  debug start).

**The exit review** (model it on GAME-08's "Phase 9's end (the exit review, 2026-10-15)": read that
section first and match its parts):
1. **The exit, met:** the Goron's Ruby (milestone 4, `Route::Ruby`); the commits.
2. **What the phase built:** the actors (from the milestones' "What was built"; ported count at the
   phase's start against its end, from COVERAGE's history or the census), the systems (bombs and
   Player's explosives, the room's wind, `EffectBlure`, the room RAM, the camera's up, the magic
   meter, the return entrances, the charged spin and combo window, the shadows the C's way, the
   texture decoder's clamp-and-mask fix, the one-points), the runs with their frames and presets,
   and the numbers (tests: 1,106 at Phase 9's end, 1,433 now; goldens 210 and 161 at
   `out/golden_base22`, now 273 and 207 at `out/golden_base30`; the menu entries the phase used).
3. **Phase 11's start: the preset.** Settle which preset Phase 11 starts from: `fairy-magic`, or
   `cavern-rooms`, or a new one. Make it whatever save the story's route leaves at the phase's end;
   milestone 5's debug-start extras (room clears, the four rupees) aren't the story's. Whatever you
   choose, check it against a run's end the way the milestones do (`ends_as_preset`), and say what
   Link has with it: the items, the songs, the hearts and the magic. Note: there's no Song of Time
   yet, so the GAME-09 milestone 5 brief's "every song up to the Song of Storms" was wrong. Update
   memory if a preset name changes.
4. **Left for later:** BACKLOG #70 to #99, which Phase 10 opened, grouped by where they'll be
   picked up (each phase's scope, the editor track, polish), as Phase 9's review did. Also the
   adult's parts left as placeholders (`En_Go2`'s adult types, `Demo_Du`'s other cutscenes: #72,
   #73, #83), the Bombchus (#70, #99), the spells (#85), the lens and the rumble (#60), and room 10's
   web question (dropped, milestone 3).
5. **Checked:** ROADMAP (Phase 10 done, its milestones' entries, the "Where things stand" prose),
   COVERAGE (`coverage.py`'s `SCENES` phase 10 set to done; regenerate with `ootx census` then
   `python scripts/coverage.py`), the ADR index (0072 to 0076, 0080), and BACKLOG all agree with
   GAME-09. Then `cargo test --release --workspace` (`--no-fail-fast`), `cargo test -p layering`,
   and `python scripts/golden.py check --bin-dir target/game26/release` (273 of 273 expected; nothing
   in the review should change a hash).

**Then Phase 11's survey** (Zora's River, Zora's Domain, Zora's Fountain and Jabu-Jabu's Belly,
**Master Quest**; see ROADMAP's "The whole game" outline and COVERAGE's Phase 11 row). Follow
ROADMAP's "How a phase starts and how a milestone is done":
- The scenes and their layers (`ootx scene-info`).
- The route a child takes from the Phase 10 end preset: the frogs, the bottle and Ruto's letter, the
  fish, Lord Jabu-Jabu, Ruto, Barinade, the Zora's Sapphire.
- What's already ported and reused, and the systems needed: water and swimming (Player's swimming
  is ported from the spikes), the boomerang, the Zora's diving game, bottles and fish, and Jabu's
  draw config.
- The sizes in lines of C.
- Things to check for me.

Then a **proposal**: milestones in the story's order with the exit early if it can be, a run for
each, and the calls that are mine. Write it as a new `docs/GAME-10-<name>.md`. Stop there for my
calls.

**Lessons (all still apply)**
- Never script an enemy fight or the long story chain, even if a brief names one. Give a debug start
  beside the enemy (a `game-*.bat` option) and put the fight on the hand-test list.
- Settle reachability only as far as you need to reach and exercise the actors. If it gets deep, note
  it in a line, give a debug start past it, and move on. Coverage matters more than completability.
- A debug start may mark a room cleared (`start_clears`) so the room's enemies stay out of a run.
- Before building a run on a save's items, check the preset's actual inventory and songs. Don't
  trust a brief's summary of it.
- In tests, place Link inside the room you loaded, or that room's objects never load (`(not
  initialised)` actors). A dyna actor's top can tie with another's (room 2's lift and stairs).
- Player can't spawn during his update (ADR 0072's predicted handle; ADR 0076's request with a
  snapshot); PlayerIface writes.
- Collider spheres updated in draw go in `draw_update`.
- Test bombs get timer 2.
- Tests must not depend on Rand luck. A newly ported actor's Rand calls move other cases' sequences.
- An ocarina route's test needs the audio side (`tests/summit.rs`'s and `cavern_rooms.rs`'s `run`).
- A song spot or a time block hears the ocarina only within its range: put Link right at it.
- Shadows come from `ActorShape_Init` (`actor.shape_init`); never draw one by hand.
- Textures and meshes are baked at import: a bake change needs a re-import.
- Goldens: prove every changed hash with a build where the cause is taken out
  (`target/game26proof`). A trace that changes from a draw-only change is a bug. Record and save the
  new base, and describe trace changes field by field.
- `cargo test --no-fail-fast`. Rebuild the sandbox and game binaries after preset changes. Don't
  rebuild `target/game26` while a golden check runs on its binaries; use another
  `CARGO_TARGET_DIR` for tests meanwhile.
- Bash heredocs choke on apostrophes, and backslashes in tool input can be eaten: write Python
  scripts with the Write tool, and use raw strings for Windows paths.
- Never `sed -i` a CRLF file. In Python on Windows always read and write in binary. Check a file's
  line endings before a byte replace.
- Check every id against its header; don't write a constant from memory. Check which XML this ROM
  uses (`baseroms/gc-eu-mq-dbg/config.yml`).
- Tests pass vacuously without a pack: check timing with `OOT_DATA_DIR` missing.
- Running `oot_sandbox` without `--screenshot` or `--trace` opens a window. Ask before opening any
  window.

**How we work**
- Port function by function from `D:/OOT Modding/oot-main` (52a510f) with the decomp's names. Cite
  constants, mark the game's own bugs "@bug (game)", and order arrays by linkAge, adult first.
- Docs: GAME-09's exit review section; ROADMAP, COVERAGE and BACKLOG kept in agreement.
- My view of the real game wins over a decomp-faithful visual. If I report something the C or the
  ROM's data does too, show me and ask.
- Commit and push only when I ask.

**Environment:** `CARGO_TARGET_DIR=target/game26`; `OOT_DATA_DIR=out/data26` (absolute for cargo
test); `OOT_SAVE_DIR=out/saves`. Background `&` and `sleep` are blocked: use `run_in_background` and
Monitor.
