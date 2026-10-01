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
| `test-mido-shop.bat` | GAME-03 milestone 3's tests: Mido (blocking, his texts and flags, stepping aside), the Kokiri shop (the shelves, browsing, buying, the rupee check, Start in the play frame), and the exit test (Link's bed on a new save to the Deku Shield, both worn, and past Mido) |
| `test-cutscenes.bat` | GAME-03 milestone 4's tests: the cutscene scripts from the pack, the spline camera, the Deku Tree's talk (yes opens his mouth, no asks again), his scene's intro, a forced text holding Link, and the exit test (Link's bed on a new save past Mido into the Deku Tree, no preset) |
| `test-opening.bat` | GAME-03 milestone 5's tests: Navi (her spawn, into Link's cap, C-Up and her text, her cues in the Deku Tree's talk), the Kokiri fairies, the opening from the file select's new file through its four cutscene layers, and Phase 4's exit test (the new file's first frame through the opening into the Deku Tree) |
| `test-audio.bat` | GAME-04 milestone 1's tests: the microcode on made-up data; every sample decoded bit for bit; a Kokiri Forest instrument's note (its volume each update against the C's ADSR, its pitch, its decay), a drum, the reverb's decay; the audio heap as the C lays it; Kokiri Forest's sequence against the extractor's reading; the pack's audio data against the ROM and the C |
| `test-music.bat` | GAME-04 milestone 2's tests, headless with the audio library offline: Kokiri Forest from a new game (its sound settings, the spec change and the music's start as the C queues them, the first `Audio_Update`'s commands, the wait for the reset, the forest's sequence playing and looping); from Link's house into the forest (the exit's fade, the music resumed where the forest left it); the forest at night (its nature ambience and critters); the audio boundary's boot view and spec changes; the pack's game audio tables and sound settings against the C and the ROM |
| `test-sfx.bat` | GAME-04 milestone 3's tests, headless with the audio library offline: walking in Kokiri Forest plays a footstep on the C's frames for the floor underfoot, and each sounds on the sound effects' sequence; the message box's passes, end and close; the pack's sound effect tables against the C |
| `test-sfx-route.bat` | GAME-04's exit test (Phase 5): the scripted runs' sound effect requests against the C's calls, frame by frame, with where each is: the Kokiri Sword's chest (the lid on its frames 30 and 90, the light's flash, the chest's and the item's fanfares), Mido's four chests, every rupee taken and counted, the wonder items' drops, Navi into Link's hat, the bushes cut (through sound sources), and doors (open, close, and the entrance's sound on the far side) |
| `golden-check.bat` | The golden renders and traces (e.g. `golden-check.bat --only spot04`). Recording new hashes stays a deliberate step: `python scripts\golden.py record`, logged in `golden\README.md` |
| `import.bat` | Imports the asset pack into the data folder (after a pack format change) |
| `game.bat` | The game: Kokiri Forest, or any flags (e.g. `game.bat --entrance ENTR_SPOT04_3`) |
| `game-links-house.bat` | The game from Link's bed, with the Deku Tree's mouth open (the playthrough's route by hand) |
| `game-deku-tree-open.bat` | The game at the Deku Tree, his mouth open (`--preset deku-tree-open`) |
| `game-deku-tree-dead.bat` | The game at the Deku Tree after Gohma (`--preset deku-tree-dead`) |
| `game-new-save.bat` | The game from Link's bed on a new save: the way to the Kokiri Sword by hand (the ramp, the crawlspace, the boulder, the chest), then 40 rupees, the shop, Mido, and the Deku Tree's talk into his mouth |
| `game-new-file.bat` | The game as the file select starts a new file (`--new-file`): the opening (the narration over Link asleep, the nightmare, Navi sent, her waking him), then Navi with Link; I (C-Up) talks to her once she calls. Enter (Start) skips a scene |
| `game-sword-chest.bat` | A shortcut: a new save (no sword, no shield) in front of the Kokiri Sword's chest (the `--room 2 --at ...` debug start): A opens it, Enter equips the sword |
| `game-deku-tree-talk.bat` | A shortcut: a new save entering the Deku Tree's meadow (`ENTR_SPOT04_1`): his talk starts at once (the cutscenes `D_808BCE20`, then `D_808BD520` on yes, `D_808BD790` on no); walk into the open mouth for the Deku Tree's intro |
| `game-shop.bat` | A shortcut: outside the Kokiri shop's door (`ENTR_SPOT04_4`) with the Kokiri Sword worn and 40 rupees (`--preset sword-and-40-rupees`): buy the Deku Shield, Enter to wear it, then east over the ford to Mido |
| `game-music.bat` | The game in Kokiri Forest with another sequence forced in place of its own (`--music 30`, the title theme: `Environment_ForcePlaySequence`); with arguments, e.g. `game-music.bat --music 62` (the Lost Woods) or `--music 85` (the shop). Every scene plays its own music by itself now (`game.bat`) |
| `game-night.bat` | The game in Kokiri Forest at 20:00 (`--time 20:00`): by night the forest plays its nature ambience (the stream, the crickets and the other critters) instead of its music. Time doesn't pass yet |
| `game-door.bat` | A shortcut: a child in a Kakariko house in front of its door (`--entrance ENTR_KAKARIKO_0 --at ...`): Space opens it; the door's sound as it swings, then in Kakariko Village again as Link walks in (Player_Init's `entranceSound`) |
| `sandbox.bat` | The dev sandbox, with any flags |
| `sandbox-playthrough.bat` | Headless: the playthrough's trace and screenshots, into `out\run\` |
| `sandbox-sword-chest.bat` | Headless: the Kokiri Sword run's trace and screenshots, into `out\run\` |
| `sandbox-mido-shop.bat` | Headless: the shop and Mido run's trace and screenshots, into `out\run\` |
| `sandbox-new-save-deku-tree.bat` | Headless: the new save's run into the Deku Tree (the talk's cutscenes, the mouth, the intro): its trace and screenshots, into `out\run\` |
| `sandbox-new-file-deku-tree.bat` | Headless: Phase 4's exit run, the new file's opening, C-Up to Navi and on into the Deku Tree: its trace and screenshots, into `out\run\` |
| `sandbox-audio-log.bat` | Headless: the new file's run (Phase 4's exit run) with the audio library offline alongside: the sequence commands and the library's commands by frame, and what each player played, into `out\run\new_file_deku_tree_audio.json`; the run's sound (about 10 minutes) as `out\run\new_file_deku_tree.wav`, then opens it. The log lists the sound effects asked for, with their names and where they are. The cutscenes have no music or sounds of their own yet |
| `sandbox-mido-shop-audio.bat` | Headless: the Mido and shop run (Phase 5's exit run) with the audio offline: its audio log into `out\run\mido_shop_audio.json` (the `mido_shop_audio` golden) and its sound as `out\run\mido_shop.wav` (about 5 minutes), then opens it |
| `ootx-sfx.bat` | Looks a sound effect up in the pack's tables (`ootx sfx`): by id (`0x2800`), name (`NA_SE_EV_DOOR_OPEN`) or part of one (`DOOR`), with its importance, parameters and the constant to paste; asks when given nothing |
| `audio-wav.bat` | Offline WAVs from the audio library (`ootx audio-wav`), into `out\audio\`: Kokiri Forest's music (75 s, past its loop), an instrument's note at C4 and C3, a drum, and the instrument's sample as stored; then opens the music. With arguments, `ootx audio-wav <args>` (`--seq`, `--font --inst --note`, `--font --drum`, `--raw`, `--seconds`, `--wav`) |

Game keys: WASD to move, Space = A, E = B, Q = Z, I/J/K/L = C, P shows the placeholders,
F1 the collision, Backspace respawns. At a crawlspace's mouth A says Enter; W crawls, S backs
out. Enter (Start) stands in for the pause menu's equipping:
it equips every owned piece of a type with nothing worn (the sword also goes on B). The play
frame reads it, as `KaleidoSetup_Update` reads Start, so it does nothing while a message box is
up. In a shop, the stick left or right browses a shelf, A chooses, B leaves. In a cutscene, A
reads its texts and answers a question (the stick picks the answer); Link is held meanwhile.
In the opening, Enter (Start) skips to the next scene (its terminator). I (C-Up) talks to Navi
when she calls (after about 30 s in one scene); otherwise it's the viewpoint in houses.
Sound: the window opens your default output device (`--no-audio` doesn't), and each scene plays
its own music, or by night its nature ambience (`--time 20:00`); `--music <n>` forces sequence n
in place of the first scene's. Link, the message box, the HUD (rupees counting, the low-health
alarm), the targeting, the chests, doors, Navi, bushes, rocks, signs, rupees and the other
ported actors have their sounds (GAME-04 milestone 3); cutscenes have no music or sounds of
their own yet.

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
