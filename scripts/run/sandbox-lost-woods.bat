@echo off
rem Headless: GAME-08 milestone 2's scripted run. Route::LostWoods from the Lost Woods' entrance with
rem Saria's Song and the slingshot (meadow-saria): Saria's Song to the Skull Kid on room 9's stump
rem (a piece of heart), the target's three bullseyes (the bigger seed bag), the memory game's three
rem rounds (a piece of heart). Writes the trace and screenshots to out\run\.
rem (The frames move when the run changes: the trace's "step" fields say where.)
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot_sandbox || exit /b 1
if not exist out\run mkdir out\run
"%BIN%\oot_sandbox.exe" --entrance ENTR_LOST_WOODS_0 --child --preset meadow-saria --script lost-woods ^
    --trace out\run\lost-woods.json --screenshot out\run\lost-woods.png --shots-at 480,1090,1320,1500,2000 %*
exit /b %errorlevel%
