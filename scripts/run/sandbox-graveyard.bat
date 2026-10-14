@echo off
rem Headless: GAME-07 milestone 6's scripted run. Route::Graveyard from the graveyard's entrance at
rem 18:30 with Zelda's Lullaby (graveyard-lullaby): its intro, a dig with Dampe, Zelda's Lullaby at
rem the royal tomb (its gravestone blown up), the tomb's Keese, its door, past the ReDeads to the
rem wall, the Sun's Song taught and its chain back to the wall (screenshots along the way). Writes
rem the trace and screenshots to out\run\. (The frames move when the run changes: the trace's "step"
rem fields say where.)
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot_sandbox || exit /b 1
if not exist out\run mkdir out\run
"%BIN%\oot_sandbox.exe" --entrance ENTR_GRAVEYARD_0 --child --preset graveyard-lullaby --time 18:30 --script graveyard ^
    --trace out\run\graveyard.json --screenshot out\run\graveyard.png --shots-at SHOTS %*
exit /b %errorlevel%
