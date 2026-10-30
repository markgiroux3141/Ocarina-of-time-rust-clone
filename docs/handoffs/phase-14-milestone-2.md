# Handoff: GAME-13 milestone 2, the Lost Woods, the meadow and the Minuet

Written 2026-10-27, at the end of the session that closed Phase 13 (milestone 4, the exit review),
surveyed Phase 14 and built its milestone 1. It's for the next session, which builds milestone 2.

## Where things stand

- **Phase 13 is done** (GAME-12's exit review, 5d00e3c). Milestone 4 (the ranch after Epona, the
  field's Poes) is d56df2a, ADR 0095.
- **Phase 14** (GAME-13, `docs/GAME-13-the-forest-temple.md`) is surveyed and decided: six
  milestones, the user took every recommendation (the doc's "The user's calls").
- **Milestone 1, the hookshot and Dampé's race, is done** (217faa7, ADR 0096; hand-tested,
  pushed): `Arms_Hook` and Player's hookshot whole, `Camera_Special5`, `En_Po_Relay`,
  `En_Honotrap`; the runs `dampe-race` and `hookshot`; the preset **`hookshot`** (`epona-won` with
  Dampé's chest opened and the hookshot on C-Left), **milestone 2's start**.
- **The fps fix** (6ec36f0, ADR 0097): the game window uses DirectX 12 on Windows
  (`eng_app::run_window`; `WGPU_BACKEND` overrides) and `GpuModel::set_draw_values` writes only
  changed uniforms. The hand tests ran at 15 fps on Vulkan before it; now hundreds.
- **Next numbers:** ADR 0098, BACKLOG #113, run-script menu 193.
- Tests: 1,882 pass, 3 ignored (two are the level editor session's). Goldens: 407 hashes, 297 cases
  (`out/golden_base48`). Pack format 37 in `out/data29`.

## The milestone

GAME-13's proposal, row 2:
- **`En_Mb` whole** (`ovl_En_Mb/z_en_mb.c`, 1,630 lines): the meadow's Moblins. Five spear
  Moblins on paths (params 0x00FF to 0x04FF: paths 0 to 4; `EnMb_IsPlayerInCorridor`'s sight) and
  one club Moblin (0x0000 at (-3, 160, -1357): the shockwave, the quake, the knockback). The Spirit
  Temple places one too (check its type at the start).
- **`En_Xc` type 6** (`z_en_xc.c`): Sheik's Minuet at the meadow's top, now a placeholder
  (`en_xc.rs:77-79, 367-372`). `func_80B3CA38` (an adult without `EVENTCHKINF_50`, else killed),
  `EnXc_MinuetCS` (Link's z < -2225: `gMeadowMinuetCs`, `EVENTCHKINF_50`,
  `Item_Give(ITEM_SONG_MINUET)`), the fall from the sky and the walk (actions 1 to 5). Her harp and
  song (actions 6 to 18) are ported already (type 2's actions 30 to 43). About 225 lines.
- **`Obj_Mure3`** (205): Kokiri Forest's rupee ring by the adult bean lift (it spawns within 1,150
  of the camera; `Item_DropCollectible2` is in `en_po_relay.rs`: move it to `en_item00.rs`).
- **The Minuet's warp:** shared with the Prelude's (`sWarpSongEntrances`, ported: to
  `ENTR_SACRED_FOREST_MEADOW_2`). A test only.
- **The hookshot up to the temple:** the meadow's exit 2 is `ENTR_FOREST_TEMPLE_0`, on a ledge
  reached by the hookshot (a tree branch: **unverified** that its poly takes the hookshot, word 1
  bit 17).

**Runs** (short debug starts, no fights: the user's standing rule):
- (a) The Lost Woods from `hookshot`: room 3's adult Mido (`En_Md`, ported) blocks the way until
  Saria's Song (`EVENTCHKINF_0A`, which the story hasn't set yet), then the meadow
  (`ENTR_SACRED_FOREST_MEADOW_0`). GAME-08's `Route::Meadow` is the child's way through the woods.
- (b) A debug start at the meadow's top: Sheik's Minuet (`EVENTCHKINF_50`, the song).
- (c) The Minuet's warp.
- (d) The hookshot up to the temple's door, into the temple.
- The maze past the Moblins is the hand test's (the user's call 5).

**Presets:** `minuet` (`hookshot` with Mido past, `EVENTCHKINF_0A`, and the Minuet,
`EVENTCHKINF_50`, the song in the quest items).

**Helpers:** at most two at once, no sub-agents. Recommended: one on `En_Mb` (the largest). The main
session keeps `En_Xc` 6 (a cutscene), `Obj_Mure3` if small enough, the runs, presets, goldens and
docs. Give the helper its own `CARGO_TARGET_DIR` (`target/game26h`) and data dir (import its own
pack: `OOT_DATA_DIR=<abs>/out/data29a target/game26h/release/oot.exe import`), the files it mustn't
edit, the "never `sed -i`" rule and the report format (what's ported, schema/bakes/characters,
edits outside its module, tests, `@bug (game)`s, what a run can read). The main session adds no
`NA_SE_*`/`NA_BGM_*` to `oot_game::audio::sfx`: actors define their sounds locally. If a helper adds
bakes, bump `pack.rs`'s `FORMAT_VERSION` once yourself (to 38) before it starts, and re-import
`out/data29` at the end (or a new `out/data30`, and change `scripts/run/_env.bat`'s `DATA_NAME`).

## Before you start: the survey's open points

- The meadow tree's hookshot poly (the run (d) depends on it): find the polys near the exit with
  `SurfaceType_CanHookshot` (`oot_game::surface::SurfaceType::can_hookshot`), as milestone 1's test
  probed Goron City's post.
- `En_Mb`'s Spirit Temple placement's type.
- How Link reaches the temple ledge in the original (the branch above the exit).

## Standing rules

From the user, in memory and the earlier briefs:
- **Commit and push only when the user asks.** Stage only this work's files. Another session works
  on the level editor in the same tree: `crates/tools/overworld*` (except `actors.rs`'s editor
  names, which are ours), `crates/engine/eng_render/src/{lib,passes,pipelines}.rs` and `shader.wgsl`
  (its clip plane), `Cargo.lock`, `docs/EDITOR-*.md`, `docs/adr/0094-*.md` and its row in
  `docs/adr/README.md`, the E5 row in `docs/ROADMAP.md`. Stage shared files by hunk:
  `git diff f > p`, cut the patch to our hunks, `git apply --cached p`. For the ADR index, stage
  exact content: `git show HEAD:docs/adr/README.md` plus our rows, `git hash-object -w`,
  `git update-index --cacheinfo`. Say whether the separation was clean.
- **File edits:** never `sed -i`. In Python, read and write bytes and keep each file's line endings.
  CRLF in the working tree: `save.rs`, `player.rs`, `lib.rs` (oot_actors), `collision_check.rs`,
  `gbi.rs`, `bgcheck.rs`, `ROADMAP.md`, `BACKLOG.md`, `adr/README.md`, `actors.rs` (the editor's),
  `characters.rs`, `menu.bat`, the `.bat`s. LF: `playthrough.rs`, `message.rs`, `camera.rs`, the
  GAME docs, `scripts/run/README.md`, `golden.py`. A byte-level replace script that converts LF
  patterns to the file's endings worked well (write it with the Write tool, specs as JSON).
- **Write scripts with the Write tool.** Bash heredocs mangle backslashes and quotes (an apostrophe
  in a heredoc'd Python broke one this session).
- **Never script enemy fights or long routes;** give debug starts and a hand-test list.
- **Ask before opening any window.** `oot`/`oot_sandbox` without `--screenshot`, `--trace` or
  `--wav` opens one (this session opened one by mistake while timing: don't).
- **Port function by function** with the decomp's names. Cite each constant with its header and
  check every id against it. Mark game bugs `@bug (game)`.
- **Each new actor needs:** a schema with Params and Flags paragraphs (ADR 0066, 0062); its texts;
  registration in `crates/game/oot_actors/src/lib.rs`'s four places; an editor name in
  `crates/tools/overworld_editor/src/actors.rs`; an `ootx` character for a new skeleton.
- **`tests/schemas.rs` must pass** (params through the schema's `Field` consts; flag calls match the
  schema, computed ones with a `// schema:` note; every text id literal in `texts` or noted).
- **Goldens:** check before recording; prove every changed hash with a build where the cause is
  taken out (`target/game26proof`: unregister the actor, build, `--only` the cases, restore); record,
  check again, save `out/golden_base49`, log it in `golden/README.md`. Don't run `cargo` in
  `target/game26` while a golden check runs. Milestone 4's proof: nine traces changed only in their
  actor counts (placeholders replaced by actors their init kills): compare traces line by line.
- **Keep the docs in agreement:** GAME-13's milestone section (the answer, what was built, results,
  corrections, known gaps, how to check, by hand) and its status table; ROADMAP's Phase 14
  paragraph; COVERAGE (`ootx census` with a rebuilt `ootx`, then `python scripts/coverage.py`);
  BACKLOG; the ADR and its index row; the golden log; the run scripts (`game-`, `test-`,
  `sandbox-` bats, CRLF), the menu and `scripts/run/README.md`; `docs/HAND-TESTS-OWED.md` only if a
  hand test is deferred past a commit.
- **Environment:** `CARGO_TARGET_DIR=target/game26`; `OOT_DATA_DIR` absolute,
  `D:/Claude Code Projects/OOT Clone/out/data29`; `OOT_SAVE_DIR=out/saves`. Decomp:
  `D:/OOT Modding/oot-main`. A new bake bumps `pack.rs`'s `FORMAT_VERSION` and needs a re-import.
- **Checks before reporting:** `cargo test --release --workspace`; `cargo test -p layering`; no
  warnings; the golden check.

## What milestone 1 taught that milestone 2 will meet

- **Runs live in `crates/game/oot_actors/src/playthrough.rs`.** A route needs: `entrance`, `preset`,
  `start` (a debug start), `time` (night), `script`, `max_frames`, `needs_audio` (any ocarina:
  Saria's Song to Mido, the Minuet), the `from_script` list, `tasks`; steps in `Step` and
  `Step::name`; a `Who` entry for a talk target. Tests drive routes as `tests/hookshot_run.rs` and
  `tests/ranch_after_runs.rs` do (an ocarina route's test gives `w.audio_side` the offline audio).
- **Useful tasks:** `Converse(Who, point, choices, until, step)` (its first stage only walks: an
  actor whose talk is auto-accepted must be approached from outside its range, as Dampé's 250);
  `Texts(until, step)` (A through boxes); `Ocarina(button, song, step)` (C-Right 2 in these presets;
  it records `Step::OcarinaOut` first); `DrawSlingshot(step)` and `ShootUntil(target, until, step)`
  aim the slingshot, the hookshot or the bow (`slingshot_button` finds the item); `ClimbTo(point,
  step)`; `Settle` waits for Player's `Action::StandingStill` on the ground.
- **The hookshot:** only some polys take it (Goron City's post: its upper band, from about 550; the
  lower part throws it back). After a flight Link catches a ledge within reach (`func_80837B9C`):
  a run then climbs (`ClimbTo`).
- **Sound and the sandbox:** golden cases render headless on wgpu's default backend (Vulkan); only
  the window uses DirectX 12. `golden.py`'s cases sit after `big_poe`/`hookshot_fly`.
- **Text ids:** print a message from the pack (`pack.messages()`: `entries`, `data`; printable ASCII
  between one entry's segment and the next) with a throwaway ignored test in `oot_game/tests`.
