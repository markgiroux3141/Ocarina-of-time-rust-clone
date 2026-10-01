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
echo     4  golden-check         The render and trace regression
echo     5  import               Re-import the asset pack
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
echo.
echo   Headless
echo    10  sandbox-playthrough  The playthrough's trace and screenshots, into out\run
echo    15  sandbox-sword-chest  The sword route's trace and screenshots, into out\run
echo    18  sandbox-mido-shop    The shop and Mido route's trace and screenshots, into out\run
echo    21  sandbox-new-save-deku-tree  The new save's run into the Deku Tree, into out\run
echo    24  sandbox-new-file-deku-tree  The new file's run: the opening, Navi, the Deku Tree, into out\run
echo    27  audio-wav            WAVs of Kokiri Forest's music, a note and a drum, into out\audio (opens the music)
echo    30  sandbox-audio-log    The new file's run with its audio log and WAV, into out\run (opens the WAV)
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
if not defined script goto menu
echo.
call "%~dp0%script%.bat"
echo.
echo %script% finished (exit code %errorlevel%).
pause
goto menu
