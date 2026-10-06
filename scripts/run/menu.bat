@echo off
rem A menu of the scripts in this folder, for double-clicking. It comes back after each one.
setlocal
:menu
cls
echo OoT clone: run scripts (settings in scripts\run\_env.bat)
echo.
echo   Build and test
echo     1  build                Build everything (release)
echo     2  test                 All the tests
echo     3  test-playthrough     GAME-02's playthrough test
echo    11  test-sword-chest     GAME-03's Kokiri Sword chest test
echo    13  test-sword-route     GAME-03's crawlspace, boulder and sword route tests
echo    16  test-mido-shop       GAME-03's Mido, shop and Deku Shield route tests
echo    19  test-cutscenes       GAME-03's cutscene tests and the new save's run into the Deku Tree
echo    22  test-opening         GAME-03's Navi and opening tests, and the new file's run into the Deku Tree
echo    25  test-audio           GAME-04's audio tests: the microcode, a note, the reverb, Kokiri Forest's sequence
echo    28  test-music           GAME-04's music tests: Kokiri Forest's music and ambience as the game starts them
echo    31  test-sfx             GAME-04's sound effect tests: footsteps, the message box
echo    32  test-sfx-route       GAME-04's exit test: the runs' sound effects against the C
echo    36  test-onepoint        GAME-04b's one-point cutscene tests: the crawlspace's exits, attention, a chest's fall
echo    38  test-camera-modes    GAME-04b's jump, climb and hang camera tests
echo    39  test-cutscene-audio  GAME-04b's cutscene music, Link's cutscene voices and modes
echo    40  test-environment     GAME-04b's rain, lightning and light override tests
echo    41  test-title-cards     GAME-04b's title card tests: Kokiri Forest's place name, the Deku Tree's intro
echo    42  test-nightmare       GAME-04b's nightmare tests: the drawbridge, the riders, the horses, the rain
echo    43  test-polish          GAME-04b's polish tests: the cutscene splines, letterbox, narration box, Link's poses
echo     4  golden-check         The render and trace regression
echo     5  import               Re-import the asset pack
echo    44  decomp-check         GAME-05's decomp upgrade check: a loose import against the old decomp's
echo    45  test-damage          GAME-05's damage and health tests: the hit kinds, death and the game over, the Deku Baba
echo    50  test-combat          GAME-05's combat tests: the guard, the battle camera, the effects, the Keese, the withered Deku Baba
echo    53  test-enemies         GAME-05's other enemies' tests: the Deku Scrubs, the Skulltulas, Gohma's larvae
echo    56  test-mechanics       GAME-05's dungeon mechanics tests: the sliding doors, switches, torches, webs, quakes, the map
echo    59  test-sticks          GAME-05's Deku Stick and props tests: the stick, the platforms, the log, the crates, the lift
echo.
echo   Play (a window; close it to come back)
echo     6  game                 Kokiri Forest
echo     7  game-links-house     From Link's bed, the Deku Tree's mouth open
echo     8  game-deku-tree-open  At the Deku Tree, his mouth open
echo     9  game-deku-tree-dead  At the Deku Tree after Gohma
echo    12  game-sword-chest     A new save at the Kokiri Sword's chest
echo    14  game-new-save        A new save from Link's bed (the sword, the shop, Mido)
echo    17  game-shop            Outside the Kokiri shop, the sword worn, 40 rupees
echo    20  game-deku-tree-talk  A new save at the Deku Tree: his talk (cutscenes), yes or no
echo    23  game-new-file        A new file as the file select starts it: the opening, then Navi
echo    26  game-music           Kokiri Forest with another sequence forced (the title theme)
echo    29  game-night           Kokiri Forest at night: its nature ambience
echo    33  game-door            A Kakariko house's door: its sounds, and Kakariko Village
echo    37  game-crawlspace      At the crawlspace's mouth: through it and out (its one-point cutscene)
echo    46  game-deku-baba       Inside the Deku Tree by a Deku Baba: its bite, its stem cut, death and the game over
echo    47  game-deku-baba-fairy The same with a fairy in a bottle: dying, it revives Link
echo    48  game-dummy           The training dummy, its touch hurting Link (asks which kind)
echo    51  game-combat          Inside the Deku Tree by a withered Deku Baba and a Keese: the shield (R), the battle camera
echo    54  game-enemies         Inside the Deku Tree by an enemy of milestone 3b (asks which)
echo    57  game-dungeon         Inside the Deku Tree at a room's debug start: the switch, the doors (asks where)
echo    60  game-sticks          Inside the Deku Tree with Deku Sticks on C-Left (J), by a torch (asks where)
echo.
echo   Headless
echo    10  sandbox-playthrough  The playthrough's trace and screenshots, into out\run
echo    15  sandbox-sword-chest  The sword route's trace and screenshots, into out\run
echo    18  sandbox-mido-shop    The shop and Mido route's trace and screenshots, into out\run
echo    21  sandbox-new-save-deku-tree  The new save's run into the Deku Tree, into out\run
echo    24  sandbox-new-file-deku-tree  The new file's run: the opening, Navi, the Deku Tree, into out\run
echo    27  audio-wav            WAVs of Kokiri Forest's music, a note and a drum, into out\audio (opens the music)
echo    30  sandbox-audio-log    The new file's run with its audio log and WAV, into out\run (opens the WAV)
echo    34  sandbox-mido-shop-audio  The Mido and shop run's audio log and WAV, into out\run (opens the WAV)
echo    35  ootx-sfx             Look a sound effect up by id or name
echo    49  sandbox-deku-baba    The Deku Baba run's trace and screenshots, into out\run
echo    52  sandbox-combat       The combat run's trace and screenshots, into out\run
echo    55  sandbox-scrub        The Mad Scrub run's trace and screenshots, into out\run
echo    58  sandbox-shutter      The switch and sliding door run's trace and screenshots, into out\run
echo    61  sandbox-stick        The Deku Stick run's trace and screenshots (the torch, the web, room 1), into out\run
echo.
echo     0  quit (or an empty line)
echo.
set "pick="
rem An empty line quits (set /p fails on it, and at the end of piped input).
set /p "pick=Choose: " || exit /b 0
set "pick=%pick: =%"
if "%pick%"=="0" exit /b 0
set "script="
if "%pick%"=="1" set "script=build"
if "%pick%"=="2" set "script=test"
if "%pick%"=="3" set "script=test-playthrough"
if "%pick%"=="4" set "script=golden-check"
if "%pick%"=="5" set "script=import"
if "%pick%"=="6" set "script=game"
if "%pick%"=="7" set "script=game-links-house"
if "%pick%"=="8" set "script=game-deku-tree-open"
if "%pick%"=="9" set "script=game-deku-tree-dead"
if "%pick%"=="10" set "script=sandbox-playthrough"
if "%pick%"=="11" set "script=test-sword-chest"
if "%pick%"=="12" set "script=game-sword-chest"
if "%pick%"=="13" set "script=test-sword-route"
if "%pick%"=="14" set "script=game-new-save"
if "%pick%"=="15" set "script=sandbox-sword-chest"
if "%pick%"=="16" set "script=test-mido-shop"
if "%pick%"=="17" set "script=game-shop"
if "%pick%"=="18" set "script=sandbox-mido-shop"
if "%pick%"=="19" set "script=test-cutscenes"
if "%pick%"=="20" set "script=game-deku-tree-talk"
if "%pick%"=="21" set "script=sandbox-new-save-deku-tree"
if "%pick%"=="22" set "script=test-opening"
if "%pick%"=="23" set "script=game-new-file"
if "%pick%"=="24" set "script=sandbox-new-file-deku-tree"
if "%pick%"=="25" set "script=test-audio"
if "%pick%"=="26" set "script=game-music"
if "%pick%"=="27" set "script=audio-wav"
if "%pick%"=="28" set "script=test-music"
if "%pick%"=="29" set "script=game-night"
if "%pick%"=="30" set "script=sandbox-audio-log"
if "%pick%"=="31" set "script=test-sfx"
if "%pick%"=="32" set "script=test-sfx-route"
if "%pick%"=="33" set "script=game-door"
if "%pick%"=="34" set "script=sandbox-mido-shop-audio"
if "%pick%"=="35" set "script=ootx-sfx"
if "%pick%"=="36" set "script=test-onepoint"
if "%pick%"=="37" set "script=game-crawlspace"
if "%pick%"=="38" set "script=test-camera-modes"
if "%pick%"=="39" set "script=test-cutscene-audio"
if "%pick%"=="40" set "script=test-environment"
if "%pick%"=="41" set "script=test-title-cards"
if "%pick%"=="42" set "script=test-nightmare"
if "%pick%"=="43" set "script=test-polish"
if "%pick%"=="44" set "script=decomp-check"
if "%pick%"=="45" set "script=test-damage"
if "%pick%"=="46" set "script=game-deku-baba"
if "%pick%"=="47" goto fairy
if "%pick%"=="48" goto dummy
if "%pick%"=="49" set "script=sandbox-deku-baba"
if "%pick%"=="50" set "script=test-combat"
if "%pick%"=="51" set "script=game-combat"
if "%pick%"=="52" set "script=sandbox-combat"
if "%pick%"=="53" set "script=test-enemies"
if "%pick%"=="54" goto enemies
if "%pick%"=="55" set "script=sandbox-scrub"
if "%pick%"=="56" set "script=test-mechanics"
if "%pick%"=="57" goto dungeon
if "%pick%"=="58" set "script=sandbox-shutter"
if "%pick%"=="59" set "script=test-sticks"
if "%pick%"=="60" goto sticks
if "%pick%"=="61" set "script=sandbox-stick"
if not defined script goto menu
echo.
call "%~dp0%script%.bat"
echo.
echo %script% finished (exit code %errorlevel%).
pause
goto menu

:fairy
call "%~dp0game-deku-baba.bat" fairy
echo game-deku-baba fairy finished (exit code %errorlevel%).
pause
goto menu

:dummy
set "kind="
set /p "kind=Kind (none, fire, ice, electric, knockback): "
call "%~dp0game-dummy.bat" %kind%
echo game-dummy finished (exit code %errorlevel%).
pause
goto menu

:enemies
set "who="
set /p "who=Enemy (scrub, hint, shop, skulltula, walltula, gold, larva): "
call "%~dp0game-enemies.bat" %who%
echo game-enemies finished (exit code %errorlevel%).
pause
goto menu

:dungeon
set "where="
set /p "where=Where (switch, lobby-top, lobby, room1 to room10): "
call "%~dp0game-dungeon.bat" %where%
echo game-dungeon finished (exit code %errorlevel%).
pause
goto menu

:sticks
set "where="
set /p "where=Where (torch, room3, room10, room5, room2): "
call "%~dp0game-sticks.bat" %where%
echo game-sticks finished (exit code %errorlevel%).
pause
goto menu
