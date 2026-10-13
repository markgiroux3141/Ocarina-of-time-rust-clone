@echo off
rem Headless: GAME-07 milestone 3's scripted run. Route::Ocarina in Hyrule Field at 18:30: the
rem ocarina out, Saria's Song (Navi calls, Saria talks), the Sun's Song (the clock run to dawn),
rem south to the fairy spot and the Song of Storms (the rain, Shot_Sun's fairy) (screenshots: the
rem staff's notes, Saria's Song's leaves, the Sun's Song's light, the storm, the end). Writes the
rem trace and screenshots to out\run\.
rem (The frames move when the run changes: the trace's "step" fields say where.)
setlocal
call "%~dp0_env.bat"
call "%~dp0_need.bat" oot_sandbox || exit /b 1
if not exist out\run mkdir out\run
"%BIN%\oot_sandbox.exe" --entrance ENTR_HYRULE_FIELD_0 --child --preset ocarina-songs --time 18:30 --script ocarina ^
    --trace out\run\ocarina.json --screenshot out\run\ocarina.png --shots-at 95,140,560,880 %*
exit /b %errorlevel%
