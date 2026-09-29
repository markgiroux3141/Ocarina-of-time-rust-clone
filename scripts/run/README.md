# Run scripts

Windows batch files for the commands used to build, test and try things. Run them from any
folder, or double-click `menu.bat` for a numbered menu. Each script sets up its own
environment and leaves your terminal as it was.

## Settings

`_env.bat` sets the two things that change between sessions:
- `TARGET_NAME`: the build folder under `target\` (`CARGO_TARGET_DIR`);
- `DATA_NAME`: the data folder under `out\` that holds the asset pack (`OOT_DATA_DIR`).

A session that builds into a new folder, or bumps the pack format and imports into a new data
folder, changes them there and nowhere else.

## The scripts

Most scripts pass extra arguments on to the program they run.

| Script | What it does |
|---|---|
| `menu.bat` | A menu of everything below |
| `build.bat` | Builds the workspace (release) |
| `test.bat` | All the tests; with arguments, `cargo test --release <args>` (e.g. `test.bat -p oot_actors --test talk`) |
| `test-playthrough.bat` | GAME-02's exit test: `Bg_Treemouth` and the playthrough |
| `test-sword-chest.bat` | GAME-03 milestone 1's exit test: the Kokiri Sword's chest on a new save, and a piece of heart |
| `test-sword-route.bat` | GAME-03 milestone 2's tests: the crawl and the crawlspace's camera, the boulder and Link's knockdown, the wonder items, and the exit test (Link's bed on a new save to the Kokiri Sword's chest, opened) |
| `golden-check.bat` | The golden renders and traces (e.g. `golden-check.bat --only spot04`). Recording new hashes stays a deliberate step: `python scripts\golden.py record`, logged in `golden\README.md` |
| `import.bat` | Imports the asset pack into the data folder (after a pack format change) |
| `game.bat` | The game: Kokiri Forest, or any flags (e.g. `game.bat --entrance ENTR_SPOT04_3`) |
| `game-links-house.bat` | The game from Link's bed, with the Deku Tree's mouth open (the playthrough's route by hand) |
| `game-deku-tree-open.bat` | The game at the Deku Tree, his mouth open (`--preset deku-tree-open`) |
| `game-deku-tree-dead.bat` | The game at the Deku Tree after Gohma (`--preset deku-tree-dead`) |
| `game-new-save.bat` | The game from Link's bed on a new save: the way to the Kokiri Sword by hand (the ramp, the crawlspace, the boulder, the chest) |
| `game-sword-chest.bat` | A shortcut: a new save (no sword, no shield) in front of the Kokiri Sword's chest (the `--room 2 --at ...` debug start): A opens it, Enter equips the sword |
| `sandbox.bat` | The dev sandbox, with any flags |
| `sandbox-playthrough.bat` | Headless: the playthrough's trace and screenshots, into `out\run\` |
| `sandbox-sword-chest.bat` | Headless: the Kokiri Sword run's trace and screenshots, into `out\run\` |

Game keys: WASD to move, Space = A, E = B, Q = Z, I/J/K/L = C, P shows the placeholders,
F1 the collision, Backspace respawns. At a crawlspace's mouth A says Enter; W crawls, S backs
out. Enter (Start) stands in for the pause menu's equipping:
it equips every owned piece of a type with nothing worn (the sword also goes on B).

## Adding a script

Each session adds scripts for what it built. Copy one that's close to the new one:

```bat
@echo off
rem What it runs, and what to look at.
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot || exit /b 1
"%BIN%\oot.exe" --entrance ENTR_... %*
exit /b %errorlevel%
```

`%BIN%` is the build's `release` folder. `_need.bat <binary>` stops with a hint if that
binary isn't built yet. Then add a line for the new script to the table above, and to
`menu.bat` if it's one to run often.

The files are CRLF (`.gitattributes`), since `cmd` can misread labels in LF batch files.
