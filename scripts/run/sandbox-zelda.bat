@echo off
rem Headless: GAME-07 milestone 4's scripted run. Route::Zelda from the castle's drain into the guards'
rem courtyard (castle-courtyard): past the patrols unseen along the south wall and up the plaza; Zelda
rem at the window, her talk (yes each time), the legend on the cutscene map, the window, Ganondorf, her
rem letter; Impa's lullaby (one wrong note, then right) and her goodbye in Hyrule Field to
rem ENTR_HYRULE_FIELD_17 (screenshots: the courtyard's patrols, Zelda's talk, Ganondorf at the window,
rem the letter, Impa's lesson, Impa in the field, the end). Writes the trace and screenshots to out\run\.
rem (The frames move when the run changes: the trace's "step" fields say where.)
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot_sandbox || exit /b 1
if not exist out\run mkdir out\run
"%BIN%\oot_sandbox.exe" --entrance ENTR_CASTLE_COURTYARD_GUARDS_DAY_0 --child --preset castle-courtyard --script zelda ^
    --trace out\run\zelda.json --screenshot out\run\zelda.png --shots-at 900,7100,10600,12200,13300,14200 %*
exit /b %errorlevel%
