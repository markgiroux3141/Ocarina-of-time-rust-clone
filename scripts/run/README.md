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
| `test-onepoint.bat` | GAME-04b milestone 1's tests, one-point cutscenes: the crawlspace's exits (9601 and 9602: the spline out on a sub camera, then the main camera back without a jump), an attention cutscene (`Camera_Demo5` into `Camera_Unique9`: its keyframes, its chime, Link held), the falling chest's shot (4500), and the one-point tables in the pack against the ROM |
| `golden-check.bat` | The golden renders and traces (e.g. `golden-check.bat --only spot04`). Recording new hashes stays a deliberate step: `python scripts\golden.py record`, logged in `golden\README.md` |
| `import.bat` | Imports the asset pack into the data folder (after a pack format change) |
| `decomp-check.bat` | GAME-05 milestone 1's pack check (ADR 0031): a loose import from `oot.toml`'s decomp, compared record by record with the old decomp's (`out\loose-2f4c25d` by default; the script says how to make it), the old names renamed through `docs\name-map`. Exits 1 while any record differs: the ones left are listed and explained in `docs\GAME-05-deku-tree.md`. `decomp-check.bat out\loose-2f4c25d --dump out\cmp` writes each differing record, both sides |
| `name-map.bat` | Regenerates `docs\name-map` (ADR 0031): `name-map.bat <old decomp> <new decomp> <baserom.z64> [<builds folder>]` builds both decomps in Docker (each with its own Dockerfile, in the volumes `oot-old` and `oot-new`, by `scripts\decomp\build_*.sh`), then pairs their names with `scripts\name_map.py build`. The checkouts are only read, and the ELFs go outside the repo (`%TEMP%\oot-decomp-builds` by default) |
| `game.bat` | The game: Kokiri Forest, or any flags (e.g. `game.bat --entrance ENTR_KOKIRI_FOREST_3`) |
| `game-links-house.bat` | The game from Link's bed, with the Deku Tree's mouth open (the playthrough's route by hand) |
| `game-deku-tree-open.bat` | The game at the Deku Tree, his mouth open (`--preset deku-tree-open`) |
| `game-deku-tree-dead.bat` | The game at the Deku Tree after Gohma (`--preset deku-tree-dead`) |
| `game-new-save.bat` | The game from Link's bed on a new save: the way to the Kokiri Sword by hand (the ramp, the crawlspace, the boulder, the chest), then 40 rupees, the shop, Mido, and the Deku Tree's talk into his mouth |
| `game-new-file.bat` | The game as the file select starts a new file (`--new-file`): the opening (the narration over Link asleep, the nightmare, Navi sent, her waking him), then Navi with Link; I (C-Up) talks to her once she calls. Enter (Start) skips a scene |
| `game-sword-chest.bat` | A shortcut: a new save (no sword, no shield) in front of the Kokiri Sword's chest (the `--room 2 --at ...` debug start): A opens it, Enter equips the sword |
| `game-deku-tree-talk.bat` | A shortcut: a new save entering the Deku Tree's meadow (`ENTR_KOKIRI_FOREST_1`): his talk starts at once (the cutscenes `gDekuTreeMeetingCs`, then `gDekuTreeMouthOpeningCs` on yes, `gDekuTreeAskAgainCs` on no); walk into the open mouth for the Deku Tree's intro |
| `game-shop.bat` | A shortcut: outside the Kokiri shop's door (`ENTR_KOKIRI_FOREST_4`) with the Kokiri Sword worn and 40 rupees (`--preset sword-and-40-rupees`): buy the Deku Shield, Enter to wear it, then east over the ford to Mido |
| `game-music.bat` | The game in Kokiri Forest with another sequence forced in place of its own (`--music 30`, the title theme: `Environment_ForcePlaySequence`); with arguments, e.g. `game-music.bat --music 62` (the Lost Woods) or `--music 85` (the shop). Every scene plays its own music by itself now (`game.bat`) |
| `game-night.bat` | The game in Kokiri Forest at 20:00 (`--time 20:00`): by night the forest plays its nature ambience (the stream, the crickets and the other critters) instead of its music. Time doesn't pass yet |
| `game-door.bat` | A shortcut: a child in a Kakariko house in front of its door (`--entrance ENTR_KAKARIKO_CENTER_GUEST_HOUSE_0 --at ...`): Space opens it; the door's sound as it swings, then in Kakariko Village again as Link walks in (Player_Init's `entranceSound`) |
| `test-camera-modes.bat` | GAME-04b milestone 2's tests: the camera modes that were on the Normal1 fallback (`Camera_Jump2` on the ladder, `Camera_Jump1` in the air, `Camera_Unique1` hanging) against the C's arithmetic on the pack's camera data, and the climbing and ledge tests |
| `test-cutscene-audio.bat` | GAME-04b milestone 3's tests: the Deku Tree's talk plays its scripts' music commands on their frames, Link's groan, sigh and slips as Navi wakes him on their animation frames, and mode 69 through Player's cutscene mode tables |
| `test-environment.bat` | GAME-04b milestone 4's tests: the nightmare's rain and lightning bolts on the C's frames, a cutscene's light setting override blending in, a lightning strike's flash |
| `test-nightmare.bat` | GAME-04b milestone 6's tests: the drawbridge lowering on the script's flag (its and its chains' steps, the torches' lights), the riders on their cues (Zelda's line, Ganondorf's rear), the horses' skins (a bone per limb and vertex group), the rain's drops, and the pack's skin skeletons against the ROM |
| `test-polish.bat` | GAME-04b milestone 7's tests: the narration's and the Deku Tree's talk's camera on their splines every frame, the letterbox's steps, the narration's text box placement, and Link's pose on each of the opening's cues |
| `test-title-cards.bat` | GAME-04b milestone 5's tests: Kokiri Forest's place name on entering (its delay, fade and rectangle against `TitleCard_Update` and `_Draw`), and the Deku Tree's intro card from its `CS_MISC` 15 |
| `test-damage.bat` | GAME-05 milestone 2's tests: each of Link's hit responses with its timers (the stagger, the knockdown, frozen, electrified, burning and the Deku Shield burnt, the swimming hit), the red flash, death through `GameOver_Update` and the game over menu's stand-in to "Continue" and the respawn, a bottled fairy's revival, `En_Dekubaba`'s damage tables, `Health_ChangeBy`, the damage table's lookup and `CollisionCheck_ApplyDamage`, the colour filter's fog, and the exit's runs (the dummy hits Link; a Deku Baba bites him, then its stem is cut) |
| `game-deku-baba.bat` | Inside the Deku Tree on its top floor, 85 from a Deku Baba (`--entrance ENTR_DEKU_TREE_0 --preset deku-tree-inside --at ...`). Stand still and it bites; Q (Z) locks on, E (B) slashes: hit it while it's stuck after a missed bite, then cut its stem for a Deku Stick. Die to see the game over: its menu isn't drawn (the stand-in, ADR 0032); at "Save?" in the console D then Space for No (or Space for Yes), at "Continue?" Space: Link starts again at the entrance. `game-deku-baba.bat fairy` adds a bottled fairy, which revives him |
| `game-dummy.bat` | Inside the Deku Tree with the training dummy 60 ahead, hurting Link when he walks into it: `game-dummy.bat none` (or nothing; the stagger), `fire` (burning, the Deku Shield burnt away), `ice` (frozen: mash Space), `electric` (shocked), `knockback` (knocked down). The red flash shows while he's invincible; E (B) slashes the dummy |
| `sandbox-deku-baba.bat` | Headless: GAME-05 milestone 2's run (the Deku Baba's bite, then its stem cut), its trace and screenshots, into `out\run\` (the `deku_baba` golden) |
| `test-combat.bat` | GAME-05 milestone 3a's tests: Link's guard (up, held, down; a blow blocked; a Deku nut bounced back; a fire blow burning the Deku Shield away), `Camera_Battle1` against the C's arithmetic, a hit mark's life frame by frame, the withered Deku Baba's and the Keese's states and damage tables, and the exit's run |
| `game-combat.bat` | Inside the Deku Tree on its first room's ground floor, by a withered Deku Baba and under a Keese on the wall (`--entrance ENTR_DEKU_TREE_0 --preset deku-tree-inside --at ...`). R guards (the stick tilts the shield); Q (Z) locks on (the battle camera), E (B) slashes, Space picks up the Deku Stick. Lock on and hold R as the Keese dives to block it, then slash it while it hovers |
| `sandbox-combat.bat` | Headless: GAME-05 milestone 3a's run (the withered Deku Baba killed and its stick taken, the Keese's dive blocked and the Keese killed), its trace and screenshots, into `out\run\` (the `combat` golden) |
| `test-enemies.bat` | GAME-05 milestone 3b's tests: the Mad Scrub (up, its spit, down; its nut bounced back, its run and death; stunned and set alight), the hint scrubs' order puzzle and talk, the Business Scrub and the salesman (no sale, then the Deku Shield sold), the Skulltula (its thread, its front and back, its bounces and death, touching Link), the Skullwalltula and the Gold Skulltula with its token, Gohma's eggs and larvae, and the exit's run |
| `game-enemies.bat` | Inside the Deku Tree from a debug start in the room of one of milestone 3b's enemies (`--room`, `--at`): `scrub` (room 4, the default), `hint` (room 9), `shop` (room 3), `skulltula` (room 5), `walltula`, `gold` and `larva` (room 0), e.g. `game-enemies.bat hint`. R guards: hold it to bounce a scrub's nut back. Q (Z) locks on, E (B) slashes, Space talks |
| `sandbox-scrub.bat` | Headless: GAME-05 milestone 3b's run (a Mad Scrub's nut bounced back off the Deku Shield, the scrub caught and slashed), its trace and screenshots, into `out\run\` (the `scrub` golden) |
| `test-mechanics.bat` | GAME-05 milestone 4a's tests: the sliding doors (through room 10's door and barred behind, unbarred by the room's clear or a switch, a small key spent, the Gohma slab), the small key counter, the floor, eye and crystal switches, the torches and a Keese set alight, the webs (bouncing, broken by a fall, burnt), Navi's hint tags, the quakes, the map and compass data, every room's debug start, and the exit's run |
| `game-dungeon.bat` | Inside the Deku Tree from a debug start (`--room`, `--at`): `switch` (room 0's top floor by the floor switch, the default), `lobby-top`, `lobby`, or `room1` to `room10` (each room's own start; room 11 is the drop to Gohma), e.g. `game-dungeon.bat room3`. Step on a switch, open a sliding door with Space (A) facing it |
| `sandbox-shutter.bat` | Headless: GAME-05 milestone 4a's run (the switch, the web burnt, through room 10's door, barred behind), its trace and screenshots, into `out\run\` (the `shutter` golden) |
| `test-sticks.bat` | GAME-05 milestone 4b's tests: the Deku Stick (out from C-Left and put away, B taking out the sword, none left, the pause menu putting them on C-Left (since 5b-1), lit at a torch and burning down, broken on a target and a wall, swimming with it lit, room 4's timed torches, room 10's chest), the torches and webs, `Bg_Ydan_Hasi` (the block, the water, the rising platforms), `Bg_Ydan_Maruta` (the log, the ladder), the crates, the lift, the fragments (`Effect_Ss_Kakera`: the bushes', rocks' and boulder's pieces), the Gold Skulltula behind its crate, the writable water boxes, and the exit's run |
| `game-sticks.bat` | Inside the Deku Tree with ten Deku Sticks on C-Left (the `deku-tree-sticks` preset) from a debug start (`--room`, `--at`, `--switch`): `torch` (room 0's middle floor by its lit golden torch, the default), `room3` (by its lit golden torch, the door to room 4 open), `room10` (by its wooden torch), `room5` (the log, the block, the water) or `room2` (on the lift), e.g. `game-sticks.bat room3`. J (C-Left) takes a stick out, J again swings it, A standing puts it away |
| `sandbox-stick.bat` | Headless: GAME-05 milestone 4b's run (a stick lit at the torch, the gap jumped, the web over room 1's door burnt, through the door), its trace and screenshots, into `out\run\` (the `stick` golden) |
| `test-push.bat` | GAME-05 milestone 4c's tests: Player's push and pull (holding on to a block, pushing it a block length frame by frame, letting go, pulling it back to its start, a gravestone pulled), room 3's push block (`Obj_Oshihiki`, `Obj_Makeoshihiki`: off the channel's end into the pit with flag 0x10 and the chime, where it spawns by its flags, the strength a block needs, a block riding on another), room 7's gravestones (`Bg_Haka`), the Song of Time's blocks (`Obj_Timeblock`, the song injected), room 2's rocks (`Obj_Bombiwa`, the explosion injected), the room travel test (0 to 10, 0 to 1, the drops 0 to 3 and 3 to 9, 9 to 11), the debug starts, the exit's run, and the engine's DynaPoly push fields |
| `game-push.bat` | Inside the Deku Tree (`deku-tree-inside`) from a debug start (`--room`, `--at`): `room3` (the upper floor behind room 3's push block, the default), `room7` (by the gravestones and the hidden stair) or `room2` (under the rocks' ledge, by the hidden blocks), e.g. `game-push.bat room7`. At the block, hold Space (A) to grab it and the stick towards it to push, away to pull |
| `sandbox-push.bat` | Headless: GAME-05 milestone 4c's run (room 3's block pushed off the upper floor into the pit, then climbed from beside it), its trace and screenshots, into `out\run\` (the `push` golden) |
| `test-slingshot.bat` | GAME-05 milestone 5a's tests: Player's Fairy Slingshot (out from C-Right and raised in first person, drawn with a seed, shot on letting go, the string's spring, the flick with no seeds, lowered, aimed Z-targeted), C-Up's first-person look, a Deku nut thrown, the pause menu putting nuts and the slingshot on C buttons (since 5b-1), the first-person draw; `En_Arrow` (seeds and nuts), `En_M_Fire1`, `Effect_Ss_Stone1` and the screen's flash; the eye switches and room 2's ladder hit by real seeds; the room travel test; the debug starts; the exit's run; the knockdown's camera (BACKLOG #18) |
| `game-slingshot.bat` | Inside the Deku Tree with the slingshot on C-Right (L), nuts on C-Down (K) and sticks on C-Left (J) (`deku-tree-slingshot`) from a debug start (`--room`, `--at`): `room1` (250 in front of room 1's eye switch, the default), `room3` (under room 3's eye switch), `room2` (in front of room 2's ladder) or `room10` (by the slingshot's chest, which drops once the room's enemies are gone), e.g. `game-slingshot.bat room3`. L takes the slingshot out and aims in first person (hold L, let go to shoot); I (C-Up) looks around |
| `sandbox-slingshot.bat` | Headless: GAME-05 milestone 5a's run (the slingshot aimed in first person at room 1's eye, a seed into it, the door to room 2 unbarred and through it), its trace and screenshots, into `out\run\` (the `slingshot` golden) |
| `test-pause.bat` | GAME-05 milestone 5b-1's tests: the pause menu (`z_kaleido_setup.c`, `z_kaleido_scope_call.c`, `z_kaleido_scope.c`'s frame, `z_kaleido_item.c`): Start opening it frame by frame, the item page's cursor and the stick's repeat, an equip with its icon flying to the C button, a wrong age's error and grey icon, a page turn, closing and the game resumed, the equipment page's stand-in, every quad baked, the scene stopped behind it; the menu's tables and the C buttons' swap; the stick and slingshot tests that equip through it; the exit's run |
| `game-pause.bat` | Inside the Deku Tree with the slingshot owned but on no button, sticks on C-Left and nuts on C-Down (`deku-tree-slingshot-owned`). Enter (Start) opens the pause menu on its item page: WASD moves the cursor, J, K and L (C-Left, C-Down, C-Right) equip the item under it, R and Q (R and Z) turn the pages (the others show their backgrounds only), Enter closes it. E (B, the save prompt, 5c) and T (L, the debug inventory editor) only log |
| `sandbox-pause.bat` | Headless: GAME-05 milestone 5b-1's run (the menu opened, the cursor to the slingshot, C-Right, R to the map page, the menu closed), its trace and screenshots, into `out\run\` (the `pause`, `pause_item` and `pause_map` goldens) |
| `game-crawlspace.bat` | A shortcut: Link in front of the crawlspace on the way to the Kokiri Sword (`--entrance ENTR_KOKIRI_FOREST_0 --at ...`). W up to it, Space (A) when it says Enter, W through it: climbing out at the far end plays its one-point cutscene (9601: the camera's spline up and out, then the normal camera without a jump); backing out the way in (S) plays 9602 |
| `sandbox.bat` | The dev sandbox, with any flags |
| `sandbox-playthrough.bat` | Headless: the playthrough's trace and screenshots, into `out\run\` |
| `sandbox-sword-chest.bat` | Headless: the Kokiri Sword run's trace and screenshots, into `out\run\` |
| `sandbox-mido-shop.bat` | Headless: the shop and Mido run's trace and screenshots, into `out\run\` |
| `sandbox-new-save-deku-tree.bat` | Headless: the new save's run into the Deku Tree (the talk's cutscenes, the mouth, the intro): its trace and screenshots, into `out\run\` |
| `sandbox-new-file-deku-tree.bat` | Headless: Phase 4's exit run, the new file's opening, C-Up to Navi and on into the Deku Tree: its trace and screenshots, into `out\run\` |
| `sandbox-audio-log.bat` | Headless: the new file's run (Phase 4's exit run) with the audio library offline alongside: the sequence commands and the library's commands by frame, and what each player played, into `out\run\new_file_deku_tree_audio.json`; the run's sound (about 10 minutes) as `out\run\new_file_deku_tree.wav`, then opens it. The log lists the sound effects asked for, with their names and where they are; the cutscenes play their own music and sounds |
| `sandbox-mido-shop-audio.bat` | Headless: the Mido and shop run (Phase 5's exit run) with the audio offline: its audio log into `out\run\mido_shop_audio.json` (the `mido_shop_audio` golden) and its sound as `out\run\mido_shop.wav` (about 5 minutes), then opens it |
| `ootx-sfx.bat` | Looks a sound effect up in the pack's tables (`ootx sfx`): by id (`0x2800`), name (`NA_SE_EV_DOOR_OPEN`) or part of one (`DOOR`), with its importance, parameters and the constant to paste; asks when given nothing |
| `audio-wav.bat` | Offline WAVs from the audio library (`ootx audio-wav`), into `out\audio\`: Kokiri Forest's music (75 s, past its loop), an instrument's note at C4 and C3, a drum, and the instrument's sample as stored; then opens the music. With arguments, `ootx audio-wav <args>` (`--seq`, `--font --inst --note`, `--font --drum`, `--raw`, `--seconds`, `--wav`) |

Game keys: WASD to move, Space = A, E = B, Q = Z, I/J/K/L = C, P shows the placeholders,
F1 the collision, Backspace respawns. At a crawlspace's mouth A says Enter; W crawls, S backs
out. Enter (Start) opens the pause menu (GAME-05 milestone 5b-1), as `KaleidoSetup_Update`
reads Start in the play frame, so not while a message box is up: WASD moves the item page's
cursor, J/K/L equip on C-Left/Down/Right, R and Q (R and Z) turn the pages, Enter closes it.
Closing it equips every owned piece of a type with nothing worn (the sword also goes on B), the
stand-in for the equipment page. In a shop, the stick left or right browses a shelf, A chooses, B leaves. In a cutscene, A
reads its texts and answers a question (the stick picks the answer); Link is held meanwhile.
In the opening, Enter (Start) skips to the next scene (its terminator). I (C-Up) talks to Navi
when she calls (after about 30 s in one scene); otherwise it's the viewpoint in houses.
Sound: the window opens your default output device (`--no-audio` doesn't), and each scene plays
its own music, or by night its nature ambience (`--time 20:00`); `--music <n>` forces sequence n
in place of the first scene's. Link, the message box, the HUD (rupees counting, the low-health
alarm), the targeting, the chests, doors, Navi, bushes, rocks, signs, rupees and the other
ported actors have their sounds (GAME-04 milestone 3); cutscenes play their scripts' music, and
Link's cutscene voices and sounds (GAME-04b milestone 3).

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
